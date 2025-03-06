pub mod convert;

pub mod auth {
    tonic::include_proto!("auth");
}

pub mod block {
    tonic::include_proto!("block");
}

pub mod block_engine {
    use crate::{sanitized::SanitizedTransactionBatch, shared::Header};
    use anyhow::{bail, Context};
    use std::time::Duration;
    tonic::include_proto!("block_engine");

    impl TryFrom<ExpiringSanitizedTransactionBatch> for super::ExpiringSanitizedTransactionBatch {
        type Error = anyhow::Error;

        fn try_from(value: ExpiringSanitizedTransactionBatch) -> Result<Self, Self::Error> {
            let ExpiringSanitizedTransactionBatch {
                header,
                batch,
                expiry_ms,
            } = value;
            let Some(Header { ts: Some(ts) }) = header else {
                bail!("missing header");
            };
            let ts = ts.try_into().context("failed to convert timestamp")?;
            let expires_at = ts + Duration::from_millis(expiry_ms as u64);
            let Some(SanitizedTransactionBatch { transactions }) = batch else {
                bail!("missing transactions");
            };

            let transactions = transactions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Self {
                ts,
                expires_at,
                transactions,
            })
        }
    }

    impl TryFrom<ExpiringPacketBatch> for super::ExpiringVersionedTransactionBatch {
        type Error = anyhow::Error;

        fn try_from(value: ExpiringPacketBatch) -> Result<Self, Self::Error> {
            let ExpiringPacketBatch {
                header,
                batch,
                expiry_ms,
            } = value;
            let Some(Header { ts: Some(ts) }) = header else {
                bail!("missing header");
            };
            let ts = ts.try_into().context("failed to convert timestamp")?;
            let expires_at = ts + Duration::from_millis(expiry_ms as u64);
            let Some(super::packet::PacketBatch { packets }) = batch else {
                bail!("missing packets");
            };

            let transactions = packets
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Self {
                ts,
                expires_at,
                transactions,
            })
        }
    }
}

pub mod bundle {
    use prost_types::Timestamp;
    use solana_sdk::transaction::VersionedTransaction;
    use std::time::SystemTime;
    tonic::include_proto!("bundle");

    impl From<Vec<VersionedTransaction>> for BundleUuid {
        fn from(transactions: Vec<VersionedTransaction>) -> Self {
            let uuid = crate::derive_bundle_id(&transactions);
            Self {
                bundle: Some(transactions.into()),
                uuid,
            }
        }
    }

    impl From<&[VersionedTransaction]> for BundleUuid {
        fn from(transactions: &[VersionedTransaction]) -> Self {
            let uuid = crate::derive_bundle_id(transactions);
            Self {
                bundle: Some(transactions.into()),
                uuid,
            }
        }
    }

    impl From<Vec<VersionedTransaction>> for Bundle {
        fn from(transactions: Vec<VersionedTransaction>) -> Self {
            Self {
                packets: transactions.into_iter().map(Into::into).collect(),
                header: Some(super::shared::Header {
                    ts: Some(Timestamp::from(SystemTime::now())),
                }),
            }
        }
    }

    impl From<&[VersionedTransaction]> for Bundle {
        fn from(transactions: &[VersionedTransaction]) -> Self {
            Self {
                packets: transactions.iter().map(Into::into).collect(),
                header: Some(super::shared::Header {
                    ts: Some(Timestamp::from(SystemTime::now())),
                }),
            }
        }
    }
}

pub mod packet {
    use crate::convert::proto_packet_from_versioned_tx;
    use anyhow::Context;
    use solana_sdk::{packet::PACKET_DATA_SIZE, transaction::VersionedTransaction};
    use std::{
        cmp::min,
        net::{IpAddr, Ipv4Addr},
    };
    tonic::include_proto!("packet");

    impl TryFrom<solana_sdk::packet::Packet> for Packet {
        type Error = anyhow::Error;

        fn try_from(p: solana_sdk::packet::Packet) -> Result<Self, Self::Error> {
            Ok(Packet {
                data: p.data(..).context("discarded packet")?.to_vec(),
                meta: Some(Meta {
                    size: p.meta().size as u64,
                    addr: p.meta().addr.to_string(),
                    port: p.meta().port as u32,
                    flags: Some(PacketFlags {
                        discard: p.meta().discard(),
                        forwarded: p.meta().forwarded(),
                        repair: p.meta().repair(),
                        simple_vote_tx: p.meta().is_simple_vote_tx(),
                        tracer_packet: p.meta().is_tracer_packet(),
                        from_staked_node: p.meta().is_from_staked_node(),
                    }),
                    sender_stake: 0,
                }),
            })
        }
    }

    impl From<VersionedTransaction> for Packet {
        fn from(value: VersionedTransaction) -> Self {
            solana_sdk::packet::Packet::from_data(None, value)
                .unwrap()
                .try_into()
                .unwrap()
        }
    }

    impl From<&VersionedTransaction> for Packet {
        fn from(value: &VersionedTransaction) -> Self {
            proto_packet_from_versioned_tx(value).expect("serializes")
        }
    }

    impl From<Packet> for solana_sdk::packet::Packet {
        fn from(p: Packet) -> Self {
            const UNKNOWN_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

            let mut data = [0; PACKET_DATA_SIZE];
            let copy_len = min(data.len(), p.data.len());
            data[..copy_len].copy_from_slice(&p.data[..copy_len]);
            let mut packet = solana_sdk::packet::Packet::new(data, solana_sdk::packet::Meta::default());
            if let Some(meta) = p.meta {
                packet.meta_mut().size = meta.size as usize;
                packet.meta_mut().addr = meta.addr.parse().unwrap_or(UNKNOWN_IP);
                packet.meta_mut().port = meta.port as u16;
                if let Some(flags) = meta.flags {
                    if flags.simple_vote_tx {
                        packet
                            .meta_mut()
                            .flags
                            .insert(solana_sdk::packet::PacketFlags::SIMPLE_VOTE_TX);
                    }
                    if flags.forwarded {
                        packet
                            .meta_mut()
                            .flags
                            .insert(solana_sdk::packet::PacketFlags::FORWARDED);
                    }
                    if flags.tracer_packet {
                        packet
                            .meta_mut()
                            .flags
                            .insert(solana_sdk::packet::PacketFlags::TRACER_PACKET);
                    }
                    if flags.repair {
                        packet.meta_mut().flags.insert(solana_sdk::packet::PacketFlags::REPAIR);
                    }
                }
            }
            packet
        }
    }

    impl TryFrom<Packet> for VersionedTransaction {
        type Error = bincode::Error;

        fn try_from(value: Packet) -> Result<Self, Self::Error> {
            let packet: solana_sdk::packet::Packet = value.into();
            packet.deserialize_slice::<VersionedTransaction, _>(..)
        }
    }
}

pub mod relayer {
    tonic::include_proto!("relayer");
}

pub mod searcher {
    tonic::include_proto!("searcher");
}

pub mod custom_searcher {
    tonic::include_proto!("custom_searcher");
}

pub mod shared {
    tonic::include_proto!("shared");
}

pub mod shredstream {
    tonic::include_proto!("shredstream");
}

pub mod sanitized {
    tonic::include_proto!("sanitized");

    use anyhow::anyhow;
    use solana_sdk::{hash::Hash, message::SimpleAddressLoader, transaction::MessageHash};

    impl TryFrom<SanitizedTransaction> for solana_sdk::transaction::SanitizedTransaction {
        type Error = anyhow::Error;

        fn try_from(value: SanitizedTransaction) -> Result<Self, Self::Error> {
            let SanitizedTransaction {
                versioned_transaction,
                message_hash,
                loaded_addresses,
            } = value;

            let versioned_transaction = bincode::deserialize(&versioned_transaction)
                .map_err(|_| anyhow!("failed to deserialize versioned_transaction"))?;
            let message_hash = Hash::from(
                TryInto::<[u8; 32]>::try_into(message_hash.as_slice())
                    .map_err(|_| anyhow!("failed to deserialize message_hash"))?,
            );
            let loaded_addresses = bincode::deserialize(&loaded_addresses)
                .map_err(|_| anyhow!("failed to deserialize loaded_addresses"))?;

            solana_sdk::transaction::SanitizedTransaction::try_create(
                versioned_transaction,
                MessageHash::Precomputed(message_hash),
                None,
                SimpleAddressLoader::Enabled(loaded_addresses),
                &Default::default(),
            )
            .map_err(|_| anyhow!("failed to create SanitizedTransaction"))
        }
    }
}

pub fn derive_bundle_id(transactions: &[solana_sdk::transaction::VersionedTransaction]) -> String {
    use digest::Digest;
    use itertools::Itertools;

    let mut hasher = sha2::Sha256::new();
    hasher.update(transactions.iter().map(|tx| tx.signatures[0]).join(","));
    format!("{:x}", hasher.finalize())
}

pub struct ExpiringSanitizedTransactionBatch {
    pub ts: std::time::SystemTime,
    pub expires_at: std::time::SystemTime,
    pub transactions: Vec<solana_sdk::transaction::SanitizedTransaction>,
}

pub struct ExpiringVersionedTransactionBatch {
    pub ts: std::time::SystemTime,
    pub expires_at: std::time::SystemTime,
    pub transactions: Vec<solana_sdk::transaction::VersionedTransaction>,
}
