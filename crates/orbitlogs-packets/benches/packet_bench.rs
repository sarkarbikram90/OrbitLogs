use criterion::{black_box, criterion_group, criterion_main, Criterion};
use orbitlogs_packets::{crc::calculate_crc16, TelemetryPacket};

fn bench_crc16(c: &mut Criterion) {
    let data = vec![0u8; 1024];
    c.bench_function("crc16_1kb", |b| {
        b.iter(|| calculate_crc16(black_box(&data)))
    });
}

fn bench_packet_parse(c: &mut Criterion) {
    // Construct a valid TelemetryPacket raw payload
    // apid = 1 (Power)
    // word1 = 0x0801 (sec_header_flag = true, apid = 1)
    // word2 = 0x0005 (seq_count = 5)
    // length = 8 (sec header) + 25 (user data) + 2 (crc) - 1 = 34
    // secondary header: timestamp_coarse (4 bytes), timestamp_fine (4 bytes)
    // user data: 25 bytes
    // crc: 2 bytes
    let mut raw_packet = vec![0x08, 0x01, 0x00, 0x05, 0x00, 0x22];
    raw_packet.extend_from_slice(&[0x60, 0x00, 0x00, 0x00]); // ts coarse
    raw_packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // ts fine
    raw_packet.push(1); // APID_POWER
    raw_packet.extend_from_slice(&[0u8; 24]); // battery_voltage etc

    // Add CRC-16
    let crc = calculate_crc16(&raw_packet);
    raw_packet.extend_from_slice(&crc.to_be_bytes());

    c.bench_function("packet_from_bytes", |b| {
        b.iter(|| {
            let res = TelemetryPacket::from_bytes(black_box(&raw_packet));
            let _ = black_box(res);
        })
    });
}

criterion_group!(benches, bench_crc16, bench_packet_parse);
criterion_main!(benches);
