use hangard::ringbuf::{RingBuf, HEADER_SIZE, MAGIC, VERSION};
use std::os::unix::fs::FileExt;

fn tmpfile(dir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(name)
}

#[test]
fn test_create_and_read_header() {
    let dir = tempfile::tempdir().unwrap();
    let path = tmpfile(&dir, "ring.bin");

    {
        let ring = RingBuf::create(&path, 1024).unwrap();
        assert_eq!(ring.head(), 0);
        assert_eq!(ring.capacity(), 1024);
    }

    let file = std::fs::File::open(&path).unwrap();
    let mut header = [0u8; 16];
    file.read_at(&mut header, 0).unwrap();

    assert_eq!(&header[0..4], &MAGIC);
    assert_eq!(header[4], VERSION);
    assert_eq!(&header[5..8], &[0u8, 0, 0]);
    let head = u64::from_le_bytes(header[8..16].try_into().unwrap());
    assert_eq!(head, 0);

    let meta = std::fs::metadata(&path).unwrap();
    assert_eq!(meta.len(), HEADER_SIZE + 1024);
}

#[test]
fn test_single_write_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = tmpfile(&dir, "ring.bin");
    let mut ring = RingBuf::create(&path, 4096).unwrap();

    let data: Vec<u8> = (0..100u8).collect();
    let (offset, len) = ring.write(&data).unwrap();

    assert_eq!(offset, 0);
    assert_eq!(len, 100);
    assert_eq!(ring.head(), 100);

    let read_back = ring.read_at(0, 100).unwrap();
    assert_eq!(read_back, data);
}

#[test]
fn test_wrap_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = tmpfile(&dir, "ring.bin");
    let mut ring = RingBuf::create(&path, 256).unwrap();

    let first: Vec<u8> = (0..200u8).collect();
    let (off1, len1) = ring.write(&first).unwrap();
    assert_eq!(off1, 0);
    assert_eq!(len1, 200);

    // Second write: 100 bytes wrapping at pos 200 (56 at end, 44 at start)
    let second: Vec<u8> = (200..255u8).chain(0..45u8).collect();
    assert_eq!(second.len(), 100);
    let (off2, len2) = ring.write(&second).unwrap();
    assert_eq!(off2, 200);
    assert_eq!(len2, 100);
    assert_eq!(ring.head(), 300);

    let read_back = ring.read_at(200, 100).unwrap();
    assert_eq!(read_back, second);
}

#[test]
fn test_head_advances_monotonically() {
    let dir = tempfile::tempdir().unwrap();
    let path = tmpfile(&dir, "ring.bin");
    let mut ring = RingBuf::create(&path, 4096).unwrap();

    let (off0, len0) = ring.write(&[0u8; 50]).unwrap();
    let (off1, len1) = ring.write(&[1u8; 75]).unwrap();
    let (off2, len2) = ring.write(&[2u8; 30]).unwrap();

    assert_eq!(off0, 0);
    assert_eq!(len0, 50);
    assert_eq!(off1, 50);
    assert_eq!(len1, 75);
    assert_eq!(off2, 125);
    assert_eq!(len2, 30);
    assert_eq!(ring.head(), 155);
}

#[test]
fn test_stale_read_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = tmpfile(&dir, "ring.bin");
    let mut ring = RingBuf::create(&path, 256).unwrap();

    // Write 300 bytes total: overwrites first 44 bytes
    ring.write(&[0xAA; 256]).unwrap();
    ring.write(&[0xBB; 44]).unwrap();

    // head = 300, capacity = 256, so offset 0 data is gone (300 - 0 = 300 > 256)
    let result = ring.read_at(0, 50);
    assert!(result.is_err(), "expected error for stale read");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("overwritten"), "expected 'overwritten' in: {}", msg);
}

#[test]
fn test_reopen_preserves_head() {
    let dir = tempfile::tempdir().unwrap();
    let path = tmpfile(&dir, "ring.bin");

    let written_head;
    {
        let mut ring = RingBuf::create(&path, 1024).unwrap();
        ring.write(&[0x42; 137]).unwrap();
        ring.sync().unwrap();
        written_head = ring.head();
    }

    let ring2 = RingBuf::open(&path).unwrap();
    assert_eq!(ring2.head(), written_head);
    assert_eq!(ring2.capacity(), 1024);
}

#[test]
fn test_partial_wrap_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = tmpfile(&dir, "ring.bin");
    let mut ring = RingBuf::create(&path, 100).unwrap();

    // Write 90 bytes filling positions 0..90
    ring.write(&[0xAA; 90]).unwrap();

    // Write 20 bytes: 10 at end (pos 90..100), 10 wrapped to start (pos 0..10)
    let chunk: Vec<u8> = (0..20u8).map(|i| 0xBB + i).collect();
    let (offset, len) = ring.write(&chunk).unwrap();
    assert_eq!(offset, 90);
    assert_eq!(len, 20);

    let read_back = ring.read_at(90, 20).unwrap();
    assert_eq!(read_back, chunk);
}
