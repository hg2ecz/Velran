use super::upload::store_single_multipart_file;
use crate::{AppFs, FsLimits, FsMode};

#[tokio::test]
async fn streams_multipart_into_confined_file() {
    let root = std::env::temp_dir().join(format!("velran-appfs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("uploads")).unwrap();
    let fs = AppFs::open_root(
        &root,
        FsMode::parse("rwc").unwrap(),
        FsLimits {
            max_file_bytes: 1024,
            ..FsLimits::default()
        },
    )
    .unwrap();
    let body = b"--X\r\nContent-Disposition: form-data; name=\"_csrf\"\r\n\r\ntoken123\r\n--X\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\nContent-Type: text/plain\r\n\r\nhello\r\n--X--\r\n";
    let result = store_single_multipart_file(
        &body[..],
        "X",
        &fs,
        "uploads/u.bin",
        body.len() as u64 + 16,
        "token123",
        "file",
        &[],
        4096,
    )
    .await
    .unwrap();
    assert_eq!(result.bytes_written, 5);
    assert_eq!(result.csrf_token, "token123");
    assert!(fs.read("uploads/u.bin").await.is_err());
    fs.commit_upload(&result, "uploads/u.bin").unwrap();
    assert_eq!(fs.read("uploads/u.bin").await.unwrap(), b"hello");
    std::fs::remove_dir_all(&root).unwrap();
}
