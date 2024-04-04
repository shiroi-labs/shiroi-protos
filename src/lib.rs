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
    tonic::include_proto!("bundle");
}

pub mod packet {
    tonic::include_proto!("packet");
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
