use bincode::serialize;
use solana_perf::packet::Packet;
use solana_sdk::transaction::{SanitizedTransaction, VersionedTransaction};

use crate::{
    packet::{Meta as ProtoMeta, Packet as ProtoPacket, PacketFlags as ProtoPacketFlags},
    sanitized::SanitizedTransaction as ProtoSanitizedTransaction,
};

pub fn packet_to_proto_packet(p: &Packet) -> Option<ProtoPacket> {
    Some(ProtoPacket {
        data: p.data(..)?.to_vec(),
        meta: Some(ProtoMeta {
            size: p.meta().size as u64,
            addr: p.meta().addr.to_string(),
            port: p.meta().port as u32,
            flags: Some(ProtoPacketFlags {
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

/// Converts a VersionedTransaction to a protobuf packet
pub fn proto_packet_from_versioned_tx(tx: &VersionedTransaction) -> Option<ProtoPacket> {
    let data = serialize(tx).ok()?;
    let size = data.len() as u64;
    Some(ProtoPacket {
        data,
        meta: Some(ProtoMeta {
            size,
            addr: "".to_string(),
            port: 0,
            flags: None,
            sender_stake: 0,
        }),
    })
}

pub fn sanitized_to_proto_sanitized(tx: SanitizedTransaction) -> Option<ProtoSanitizedTransaction> {
    let versioned_transaction = bincode::serialize(&tx.to_versioned_transaction()).ok()?;
    let message_hash = tx.message_hash().to_bytes().to_vec();
    let loaded_addresses = bincode::serialize(&tx.get_loaded_addresses()).ok()?;

    Some(ProtoSanitizedTransaction {
        versioned_transaction,
        message_hash,
        loaded_addresses,
    })
}
