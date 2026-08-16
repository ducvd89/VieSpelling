//! Soi bảng vần bằng sách thật: liệt kê những tiếng bị coi là **sai cấu tạo**,
//! xếp theo số lần gặp.
//!
//! ```text
//! cargo run --release -p giaodien --example soi_van -- sach1.epub sach2.epub
//! ```
//!
//! # Vì sao cần công cụ này
//!
//! Bảng vần trong `am_tiet.rs` là danh sách gõ tay. Thiếu một vần thì hậu quả
//! **ngược hẳn** với thứ ứng dụng này sinh ra để làm: mọi tiếng mang vần ấy đều
//! bị coi là sai, rồi bị sửa thành một tiếng khác. Tức là lấy chữ đúng của tác
//! giả đổi thành chữ sai — tệ hơn nhiều so với việc bỏ sót một lỗi.
//!
//! Lỗi ấy không tự lộ ra: bài kiểm đơn vị chỉ kiểm những vần mà người viết bài
//! kiểm nghĩ ra được, mà vần bị quên thì đúng là vần không ai nghĩ tới. Đã dẫm
//! phải hai lần — thiếu `ec` và `ic`, nên `méc`, `éc`, `xéc`, `híc`, `phéc` bị
//! báo sai.
//!
//! # Đọc kết quả thế nào
//!
//! **Số lần gặp là tất cả.** Một tiếng sai chính tả thật thì xuất hiện một hai
//! lần trong cả cuốn sách. Một tiếng xuất hiện vài chục lần mà bị báo sai thì
//! gần như chắc chắn là **bảng vần thiếu**, không phải sách sai. Nên đọc từ
//! trên xuống, và dừng lại ở chỗ số lần rơi về 1–2.

use chinhta::{am_tiet, tach_tu};
use sach::{quet, Epub};
use std::collections::HashMap;

fn main() {
    let sach: Vec<String> = std::env::args().skip(1).collect();
    if sach.is_empty() {
        eprintln!("dùng: soi_van <sach.epub> [sach2.epub …]");
        std::process::exit(1);
    }

    let mut dem: HashMap<String, usize> = HashMap::new();
    let mut tong = 0usize;
    for s in &sach {
        let Ok(epub) = Epub::nap(std::path::Path::new(s)) else {
            eprintln!("bỏ qua {s}");
            continue;
        };
        for i in epub.chi_so_van_ban() {
            let Some(noi_dung) = sach::doc_chuoi(&epub.muc[i].noi_dung) else { continue };
            for d in quet::quet(&noi_dung) {
                // **Dựng lại NFC trước khi cắt từ.** Bỏ bước này thì một chữ gõ
                // rời (`ấ` = a + mũ + sắc) bị dấu tổ hợp cắt đôi ngay giữa từ,
                // vì dấu tổ hợp không phải chữ cái — `tấn` ra thành `tâ` và `n`,
                // rồi `tâ` bị báo là vần sai. Bản đo đầu tiên của công cụ này
                // quên bước ấy và cho ra 291 lần `tâ` đứng đầu bảng, che mất
                // những lỗ hổng thật.
                let chu = chinhta::chuan_hoa::dung_lai_nfc(&d.chu).unwrap_or_else(|| d.chu.clone());
                for t in tach_tu::cat(&chu) {
                    // Chỉ xét chữ **có dấu tiếng Việt**: đó là tập mà bộ kiểm
                    // thật sự đụng tới. Chữ không dấu lẫn với tiếng Anh nên
                    // không nói lên điều gì về bảng vần.
                    if tach_tu::dang_tu(t.chu) != tach_tu::DangTu::TiengViet {
                        continue;
                    }
                    tong += 1;
                    if !am_tiet::hop_le(t.chu) {
                        *dem.entry(t.chu.to_lowercase()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut v: Vec<(String, usize)> = dem.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let bi_bao_sai: usize = v.iter().map(|(_, n)| n).sum();
    println!(
        "{tong} tiếng có dấu, {bi_bao_sai} bị báo sai ({:.3}%), {} dạng khác nhau\n",
        bi_bao_sai as f64 / tong.max(1) as f64 * 100.0,
        v.len()
    );
    println!("{:>6}  {:<14}  vần bị bác", "lần", "tiếng");
    for (chu, n) in v.iter().take(60) {
        // In luôn phần vần để dò thẳng vào bảng: bỏ dấu thanh rồi cắt âm đầu.
        let khung: String = chu.chars().map(|c| am_tiet::bo_thanh(c).0).collect();
        let van = am_tiet::tat_ca_am_dau()
            .iter()
            .find(|ad| khung.starts_with(*ad))
            .map(|ad| &khung[ad.len()..])
            .unwrap_or(&khung);
        println!("{n:>6}  {chu:<14}  {van}");
    }
}
