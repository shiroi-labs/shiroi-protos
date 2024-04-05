pub mod convert;

pub mod auth {
    tonic::include_proto!("auth");
}

pub mod block {
    tonic::include_proto!("block");
}

pub mod block_engine {
    tonic::include_proto!("block_engine");
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
}

pub mod packet {
    use anyhow::Context;
    use solana_sdk::transaction::VersionedTransaction;
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
}

pub mod relayer {
    tonic::include_proto!("relayer");
}

pub mod searcher {
    tonic::include_proto!("searcher");
}

pub mod shared {
    tonic::include_proto!("shared");
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
