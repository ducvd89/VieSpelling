//! Dựng hai bảng dữ liệu từ các file từ điển JSONL.
//!
//! ```text
//! cargo run --release -p chinhta --example dung_tu_dien -- <file.txt>...
//! ```
//!
//! Ghi ra `du-lieu/am-tiet.txt` và `du-lieu/tu-ghep.txt`. Hai file **kết quả**
//! ấy mới được commit; file từ điển gốc thì không — chúng là dữ liệu của người
//! khác, tổng gần 7 MB, và ta chỉ cần phần đã chắt ra.
//!
//! # Hai bảng, hai việc khác nhau
//!
//! **Kho âm tiết** trả lời "tiếng này có thật không". Nó mạnh hơn phép kiểm cấu
//! tạo ở chỗ bắt được tiếng đúng cấu tạo mà không tồn tại (`ngìn`), và an toàn
//! hơn ở chỗ không phụ thuộc vào bảng vần gõ tay — bảng ấy đã bỏ sót `ec`, `ic`,
//! `êc` một lần rồi, và mỗi vần bỏ sót là hàng chục chữ đúng bị sửa hỏng.
//!
//! **Kho từ ghép** trả lời "hai tiếng này có đi với nhau không". Đây mới là thứ
//! chữa được lớp lỗi mà mô hình ngôn ngữ đang chọn sai: `chúg ta` thì ứng viên
//! `chúng` ghép với `ta` thành một từ có trong từ điển, còn `chừ` thì không.
//! Bằng chứng ấy rẻ, chắc chắn, và không cần card đồ hoạ.

use std::collections::BTreeSet;
use std::io::Write;

fn main() {
    let dau_vao: Vec<String> = std::env::args().skip(1).collect();
    if dau_vao.is_empty() {
        eprintln!("dùng: dung_tu_dien <tu-dien.txt>…");
        std::process::exit(1);
    }

    let mut am_tiet: BTreeSet<String> = BTreeSet::new();
    let mut tu_ghep: BTreeSet<String> = BTreeSet::new();
    #[allow(unused_mut)]
    let mut tong = 0usize;
    let mut bo = 0usize;

    for f in &dau_vao {
        let Ok(noi_dung) = std::fs::read_to_string(f) else {
            eprintln!("không đọc được {f}");
            continue;
        };
        for dong in noi_dung.lines() {
            let Some(chu) = lay_text(dong) else { continue };
            tong += 1;
            // Wiktionary lẫn trang bản mẫu (`Bản mẫu:…`, `Thành viên:…`). Ít
            // thôi nhưng chúng không phải từ.
            if chu.contains(':') {
                bo += 1;
                continue;
            }
            // Cắt theo đúng cách [`chinhta::tach_tu`] cắt văn bản, để khoá tra
            // cứu lúc chạy khớp với khoá lúc dựng bảng. Gạch nối là ranh giới
            // tiếng, nên `a-ba-giua` thành ba tiếng.
            let tieng: Vec<String> = chinhta::tach_tu::cat(&chu)
                .iter()
                .map(|t| t.chu.to_lowercase())
                .filter(|t| !t.is_empty())
                .collect();
            if tieng.is_empty() {
                bo += 1;
                continue;
            }
            for t in &tieng {
                am_tiet.insert(t.clone());
            }

            if tieng.len() > 1 {
                tu_ghep.insert(tieng.join(" "));
            }
        }
    }

    // Lọc lỗi chính tả lẫn trong chính từ điển.
    //
    // Ba bộ nguồn đều có mục hỏng — `thuơng`, `lưòi`, `ngườì`, `dướỉ`. Để
    // nguyên thì tai hại: từ điển là phép kiểm chính, nên một lỗi nằm trong đó
    // là một lỗi ứng dụng **vĩnh viễn không bắt được nữa**.
    //
    // Phép phân: mục nào **sai cấu tạo** mà lại **sinh ra được cách sửa hợp lệ**
    // thì là lỗi chính tả. `thuơng` sửa thành `thương` nên loại; `bêtông`,
    // `micrô`, `rađa` sai cấu tạo nhưng không sửa thành gì cả — chúng là từ mượn
    // thật, giữ lại.
    //
    // Đếm số nguồn chứa mục ấy thì **không** phân được: `bêtông` và `thuơng`
    // đều chỉ có ở một nguồn.
    let mut bo_vi_sai: Vec<String> = Vec::new();
    am_tiet.retain(|t| {
        if chinhta::am_tiet::hop_le(t) {
            return true;
        }
        // Chỉ xét mục **mang dấu tiếng Việt**. Mục thuần ASCII (`abc`, `atm`,
        // `adn`, `ac`) là viết tắt hoặc ký hiệu; sửa chính tả cho chúng không
        // phải việc của ứng dụng này, mà mặc định nó cũng không đụng vào chữ
        // không dấu.
        if !t.chars().any(|c| !c.is_ascii_alphanumeric()) {
            return true;
        }
        // Và chỉ khi có cách sửa **giữ nguyên bộ khung chữ cái** — tức là chỉ
        // khác nhau ở dấu. Đó mới đúng hình dạng của một lỗi gõ dấu.
        //
        // Không siết chỗ này thì bộ lọc ăn cả từ mượn thật: phép sinh ứng viên
        // có cả xoá chữ, nên `alô` "sửa" được thành `lô`, `axit` thành `xít`,
        // `balô` thành `bảo`. Bản lọc đầu tiên loại nhầm 607 mục theo đúng kiểu
        // ấy.
        let khung = chinhta::am_tiet::bo_het_dau(t);
        let co_cach_sua = chinhta::ung_vien::sinh(t)
            .iter()
            .any(|u| chinhta::am_tiet::bo_het_dau(&u.chu) == khung);
        if !co_cach_sua {
            return true;
        }
        bo_vi_sai.push(t.clone());
        false
    });
    // Từ ghép chứa tiếng vừa loại cũng phải đi theo, không thì `khop_hang_xom`
    // lại xác nhận cho chính cái sai.
    let sai: BTreeSet<&String> = bo_vi_sai.iter().collect();
    tu_ghep.retain(|c| !c.split(' ').any(|t| sai.contains(&t.to_string())));
    eprintln!(
        "loại {} âm tiết là lỗi chính tả lẫn trong từ điển, ví dụ: {}",
        bo_vi_sai.len(),
        bo_vi_sai.iter().take(25).cloned().collect::<Vec<_>>().join(" ")
    );

    ghi("du-lieu/am-tiet.txt", &am_tiet);
    ghi("du-lieu/tu-ghep.txt", &tu_ghep);
    eprintln!(
        "{tong} mục đọc vào, {bo} bỏ qua → {} âm tiết, {} từ ghép",
        am_tiet.len(),
        tu_ghep.len()
    );

    // Soi ngược: những âm tiết mà bộ kiểm cấu tạo bác bỏ. Từ điển là chuẩn nên
    // mỗi dòng ở đây là một lỗ hổng của bảng vần — hoặc một mục lạ của từ điển.
    let vo_ly: Vec<&String> =
        am_tiet.iter().filter(|t| !chinhta::am_tiet::hop_le(t)).take(40).collect();
    if !vo_ly.is_empty() {
        eprintln!(
            "\n{} âm tiết trong từ điển mà bảng vần bác bỏ (40 mục đầu):\n  {}",
            am_tiet.iter().filter(|t| !chinhta::am_tiet::hop_le(t)).count(),
            vo_ly.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")
        );
    }
}

/// Lấy giá trị của khoá `"text"` mà không kéo cả thư viện JSON vào.
///
/// Định dạng cố định một dòng một đối tượng phẳng, nên cắt chuỗi là đủ. Kéo
/// `serde_json` vào `chinhta` chỉ để chạy một công cụ dựng dữ liệu thì crate lõi
/// phải mang thêm phụ thuộc suốt đời.
fn lay_text(dong: &str) -> Option<String> {
    let dau = dong.find("\"text\"")?;
    let sau = dong[dau + 6..].find(':')? + dau + 7;
    let mo = dong[sau..].find('"')? + sau + 1;
    let mut ra = String::new();
    let mut thoat = false;
    for c in dong[mo..].chars() {
        if thoat {
            ra.push(match c {
                'n' => '\n',
                't' => '\t',
                k => k,
            });
            thoat = false;
        } else if c == '\\' {
            thoat = true;
        } else if c == '"' {
            return Some(ra);
        } else {
            ra.push(c);
        }
    }
    None
}

fn ghi(duong_dan: &str, tap: &BTreeSet<String>) {
    let mut f = std::fs::File::create(duong_dan)
        .unwrap_or_else(|e| panic!("không ghi được {duong_dan}: {e}"));
    for t in tap {
        writeln!(f, "{t}").unwrap();
    }
}
