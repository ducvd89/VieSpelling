//! Chép các DLL runtime của CUDA sang cạnh file thực thi.
//!
//! Bản dựng có CUDA liên kết **động** vào cuBLAS, nên `vie-spellcheck.exe` cần
//! `cublas64_*.dll`, `cublasLt64_*.dll` và `cudart64_*.dll` lúc chạy. Thiếu
//! chúng thì Windows chặn ngay ở bước nạp ảnh: hộp thoại đỏ "The code execution
//! cannot proceed because cublas64_13.dll was not found", chưa vào được dòng
//! `main` nào nên không có chỗ nào để báo lỗi tử tế hơn.
//!
//! Đây **không** phải chuyện chỉ thiếu PATH. CUDA 13 dời DLL runtime từ
//! `bin\` sang `bin\x64\`, nên cách quen thuộc là cho `%CUDA_PATH%\bin` vào PATH
//! giờ không còn ăn thua — kể cả trên máy vừa cài CUDA xong. Mà dựa vào PATH thì
//! bản đóng gói đưa cho người khác cũng chết y hệt.
//!
//! Chép sang cạnh exe giải quyết cả hai: Windows tìm DLL ở thư mục chứa exe
//! trước khi tra PATH, nên thư mục build tự nó đã là bản chạy được, và đóng gói
//! chỉ là chép cả thư mục. NVIDIA cho phép phát hành lại mấy DLL này kèm ứng
//! dụng (điều khoản redistributable của CUDA Toolkit).

use std::path::{Path, PathBuf};

/// Tiền tố tên các DLL cần chép. Đuôi là số phiên bản CUDA (`_13`, `_12`) nên
/// khớp theo tiền tố thay vì gán cứng cả tên.
const CAN_CHEP: [&str; 3] = ["cublas64_", "cublasLt64_", "cudart64_"];

fn main() {
    println!("cargo:rerun-if-env-changed=CUDA_PATH");
    // Không dựng CUDA thì chẳng có gì để chép.
    if std::env::var_os("CARGO_FEATURE_CUDA").is_none() && !co_cuda() {
        return;
    }
    let Some(nguon) = thu_muc_dll() else {
        println!(
            "cargo:warning=không tìm thấy DLL runtime của CUDA; \
             vie-spellcheck.exe sẽ không chạy được cho tới khi chúng nằm cạnh nó"
        );
        return;
    };
    let Some(dich) = thu_muc_exe() else { return };

    for tien_to in CAN_CHEP {
        match tim_theo_tien_to(&nguon, tien_to) {
            Some(f) => {
                let ten = f.file_name().unwrap();
                let tai = dich.join(ten);
                // Chép lại chỉ khi khác — mỗi DLL vài trăm megabyte, chép mù mỗi
                // lần build thì `cargo build` không-đổi-gì cũng mất vài giây.
                if !cung_co(&f, &tai) {
                    if let Err(e) = std::fs::copy(&f, &tai) {
                        println!("cargo:warning=không chép được {}: {e}", ten.to_string_lossy());
                    }
                }
            }
            None => println!("cargo:warning=thiếu {tien_to}*.dll trong {}", nguon.display()),
        }
    }
}

/// Feature `cuda` khai ở crate `mohinh` nên biến `CARGO_FEATURE_CUDA` không tới
/// được build script này. Suy gián tiếp: có `CUDA_PATH` thì coi như đang dựng có
/// CUDA. Đoán sai theo hướng này vô hại — chỉ là chép thêm mấy DLL không dùng.
fn co_cuda() -> bool {
    std::env::var_os("CUDA_PATH").is_some()
}

/// CUDA 13 đặt DLL ở `bin\x64`, các bản trước đặt ở `bin`. Thử cả hai.
fn thu_muc_dll() -> Option<PathBuf> {
    let goc = PathBuf::from(std::env::var_os("CUDA_PATH")?);
    [goc.join("bin").join("x64"), goc.join("bin")]
        .into_iter()
        .find(|d| tim_theo_tien_to(d, "cublas64_").is_some())
}

fn tim_theo_tien_to(thu_muc: &Path, tien_to: &str) -> Option<PathBuf> {
    std::fs::read_dir(thu_muc).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(tien_to) && n.ends_with(".dll"))
    })
}

/// Thư mục chứa file thực thể, suy từ `OUT_DIR`.
///
/// Cargo không cho build script biết thẳng chỗ ấy. `OUT_DIR` có dạng
/// `target/<profile>/build/<crate>-<hash>/out`, nên lùi bốn cấp là ra
/// `target/<profile>` — chỗ exe nằm.
fn thu_muc_exe() -> Option<PathBuf> {
    let out = PathBuf::from(std::env::var_os("OUT_DIR")?);
    let d = out.ancestors().nth(3)?.to_path_buf();
    if d.join("build").is_dir() {
        Some(d)
    } else {
        None
    }
}

/// Hai file cùng kích thước và cùng thời điểm sửa thì coi như một.
fn cung_co(a: &Path, b: &Path) -> bool {
    let (Ok(x), Ok(y)) = (std::fs::metadata(a), std::fs::metadata(b)) else {
        return false;
    };
    x.len() == y.len() && x.modified().ok() == y.modified().ok()
}
