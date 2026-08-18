//! Bản dòng lệnh. Cùng lõi với bản cửa sổ.
//!
//! ```text
//! vsc sach.epub [-o ra.epub] [-m mo-hinh.gguf] [--kho] [--cho-trong] [-n <ngưỡng>]
//! ```
//!
//! `--kho` (khô) chạy hết mọi tầng rồi in báo cáo mà **không ghi file nào** —
//! cách an toàn để xem bộ sửa định làm gì với một cuốn sách trước khi cho nó
//! động vào.
//!
//! `--cho-trong` đổi lối hỏi mô hình: khoét chữ sai thành chỗ trống rồi chấm
//! phần điền vào, thay vì chấm cả câu — và nó tự kéo ngưỡng về con số đo cho lối
//! ấy. `-n` đặt ngưỡng bằng tay. Hai cờ này để **đo lại** khi đổi mô hình hay đổi
//! loại sách; xem [`chinhta::soat::KieuCham`]. Chỉ có ở bản dòng lệnh, bản cửa sổ
//! giữ đúng một lối.

use anyhow::{bail, Result};
use std::path::PathBuf;
use ungdung::nhat_ky::{Bao, Muc, Tin};
use ungdung::{bao_cao, cai_dat::CaiDat, xu_ly};

fn main() {
    if let Err(e) = chay() {
        eprintln!("Lỗi: {e:#}");
        std::process::exit(1);
    }
}

fn chay() -> Result<()> {
    let mut doi_so = std::env::args().skip(1);
    let mut vao: Option<PathBuf> = None;
    let mut ra: Option<PathBuf> = None;
    let mut mo_hinh: Option<PathBuf> = None;
    let mut kho = false;
    let mut chi_tiet = false;
    let mut cho_trong = false;
    let mut nguong: Option<f32> = None;

    while let Some(a) = doi_so.next() {
        match a.as_str() {
            "-o" => ra = doi_so.next().map(PathBuf::from),
            "-m" => mo_hinh = doi_so.next().map(PathBuf::from),
            "--kho" => kho = true,
            "-v" => chi_tiet = true,
            "--cho-trong" => cho_trong = true,
            "-n" => {
                let Some(v) = doi_so.next().and_then(|s| s.parse::<f32>().ok()) else {
                    bail!("-n cần một con số, ví dụ `-n 0.05`");
                };
                nguong = Some(v);
            }
            "-h" | "--help" => {
                println!(
                    "vsc <sach.epub> [-o <ra.epub>] [-m <mo-hinh.gguf>] [--kho] [-v] \
                     [--cho-trong] [-n <ngưỡng>]"
                );
                return Ok(());
            }
            _ => vao = Some(PathBuf::from(a)),
        }
    }
    let Some(vao) = vao else { bail!("thiếu đường dẫn file EPUB") };
    if !vao.exists() {
        bail!("không thấy {}", vao.display());
    }
    let ra = ra.unwrap_or_else(|| xu_ly::ten_ra(&vao));
    if ra == vao {
        bail!("file ra trùng file vào — bản gốc phải giữ nguyên");
    }

    let mh = match &mo_hinh {
        None => None,
        Some(p) => {
            if !mohinh::la_gguf(p) {
                bail!("{} không phải file GGUF", p.display());
            }
            eprintln!("Đang nạp mô hình…");
            let m = mohinh::MoHinh::nap(p)?;
            eprintln!("Mô hình: {}", m.mo_ta());
            Some(m)
        }
    };

    // Chạy khô thì vẫn phải ghi ra một chỗ nào đó vì đường đi có bước ghi file.
    // Ghi vào thư mục tạm rồi xoá — như thế bản chạy khô đi đúng mọi bước mà
    // bản thật đi, kể cả bước dễ hỏng nhất là dựng lại file zip.
    let dich = if kho {
        std::env::temp_dir().join("vsc-chay-kho.epub")
    } else {
        ra.clone()
    };

    let mut tuy_chon = CaiDat::default().thanh_tuy_chon();
    if cho_trong {
        tuy_chon.kieu_cham = chinhta::soat::KieuCham::ChoTrong;
        // Ngưỡng đi kèm lối chấm. Đổi lối mà giữ ngưỡng của lối kia thì van an
        // toàn lệch thang đo chừng 1,3 lần — xem `TuyChon::nguong_mo_hinh`.
        tuy_chon.nguong_mo_hinh = 0.018;
    }
    if let Some(n) = nguong {
        tuy_chon.nguong_mo_hinh = n;
    }
    if mh.is_some() {
        eprintln!(
            "Lối chấm: {} — ngưỡng {:.3} nats/ký tự",
            if cho_trong { "điền chỗ trống" } else { "cả câu" },
            tuy_chon.nguong_mo_hinh
        );
    }

    let bat_dau = std::time::Instant::now();
    // Nhật ký ra **stderr**, báo cáo ra stdout — nên `vsc ... > bao-cao.txt` lấy
    // đúng báo cáo mà vẫn nhìn được tiến độ trên màn hình.
    let mut day = |t: Tin| match t {
        Tin::TienDo { ty_le, mo_ta } => {
            eprint!("\r{:>3}%  {:<52}", (ty_le * 100.0) as u32, mo_ta);
        }
        Tin::Ghi(d) => {
            if d.muc != Muc::ChiTiet || chi_tiet {
                eprintln!("\r{:<60}\r[{:7.2}s] {}{}", "", d.giay, d.muc.dau(), d.chu);
            }
        }
    };
    let mut bao = Bao::moi(&mut day);
    let kq = xu_ly::xu_ly(
        &vao,
        &dich,
        tuy_chon,
        mh.as_ref().map(|m| m as &dyn chinhta::soat::ChamDiem),
        &mut bao,
    )?;
    drop(bao);
    eprintln!("\r{:<60}", "");

    print!("{}", bao_cao::chu(&kq));
    if chi_tiet {
        for s in kq.da_sua.iter().take(200) {
            println!("  [{}] {} → {}  ({})", s.loai.ten(), s.goc, s.thay_bang, s.ly_do);
        }
    }
    println!("Mất {:.1} giây.", bat_dau.elapsed().as_secs_f32());
    if let Some(m) = &mh {
        println!("Mô hình chạy {} lượt chấm.", m.so_luot.get());
    }

    if kho {
        let _ = std::fs::remove_file(&dich);
        println!("(chạy khô — không ghi file nào)");
    } else {
        println!("Đã ghi {}", ra.display());
        let bc = ra.with_extension("bao-cao.html");
        std::fs::write(&bc, bao_cao::html(&kq))?;
        println!("Báo cáo {}", bc.display());
    }
    Ok(())
}
