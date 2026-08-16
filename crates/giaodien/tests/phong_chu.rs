//! Kiểm phông chữ giao diện có đủ chữ tiếng Việt.
//!
//! Đây là rủi ro lớn nhất của phần cửa sổ, và là loại rủi ro **không bài kiểm
//! logic nào chạm tới**: chương trình chạy đúng, sửa đúng, ghi file đúng — chỉ
//! có điều mọi chữ `ệ ộ ợ ự ấ ầ` trên màn hình hiện ra thành ô vuông trống. Nó
//! chỉ lộ ra khi có người mở ứng dụng lên và nhìn.
//!
//! Phông đi kèm egui phủ Latin-1 và Latin Extended-A, nhưng **không** phủ Latin
//! Extended Additional (U+1EA0–U+1EF9) — đúng cái khối chứa hơn nửa số chữ có
//! dấu của tiếng Việt. Nên `main.rs` mượn phông của hệ điều hành, và bài kiểm
//! này canh cái danh sách mượn ấy: phông đầu tiên tìm thấy trên máy phải có đủ
//! glyph, không thì đổi thứ tự trong danh sách là hỏng giao diện mà không ai hay.

use ab_glyph::{Font, FontRef};

/// Danh sách phông trong `main.rs`. Chép sang đây vì `main.rs` là binary, không
/// `use` được từ bài kiểm tích hợp — nên bài kiểm cũng canh luôn việc hai bên
/// khớp nhau bằng cách đọc thẳng file nguồn ở [`danh_sach_khop_ma_nguon`].
const UNG_VIEN: [&str; 5] = [
    r"C:\Windows\Fonts\segoeui.ttf",
    r"C:\Windows\Fonts\tahoma.ttf",
    r"C:\Windows\Fonts\arial.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
];

/// Mỗi chữ đại diện cho một khối Unicode mà tiếng Việt cần.
const PHAI_CO: [(char, &str); 8] = [
    ('đ', "Latin Extended-A"),
    ('ă', "Latin Extended-A"),
    ('ơ', "Latin Extended-B"),
    ('ư', "Latin Extended-B"),
    ('ệ', "Latin Extended Additional"),
    ('ộ', "Latin Extended Additional"),
    ('ợ', "Latin Extended Additional"),
    ('ự', "Latin Extended Additional"),
];

#[test]
fn phong_dau_tien_tim_thay_co_du_chu_viet() {
    let Some((duong_dan, byte)) =
        UNG_VIEN.iter().find_map(|p| std::fs::read(p).ok().map(|b| (*p, b)))
    else {
        // Máy dựng không có phông nào trong danh sách. Không kết luận được gì,
        // nhưng cũng không được báo xanh giả — nói rõ ra.
        eprintln!("bỏ qua: máy này không có phông nào trong danh sách");
        return;
    };
    let phong = FontRef::try_from_slice(&byte)
        .unwrap_or_else(|e| panic!("{duong_dan} không đọc được như một phông: {e}"));

    let mut thieu = Vec::new();
    for (c, khoi) in PHAI_CO {
        // `glyph_id` trả về 0 khi phông không có ký tự ấy — đó chính là glyph
        // "ô vuông trống" mà người dùng sẽ nhìn thấy.
        if phong.glyph_id(c).0 == 0 {
            thieu.push(format!("{c} (U+{:04X}, {khoi})", c as u32));
        }
    }
    assert!(thieu.is_empty(), "{duong_dan} thiếu glyph: {}", thieu.join(", "));
}

/// Mọi ký tự ngoài ASCII mà giao diện thật sự vẽ đều phải có glyph.
///
/// Bài kiểm trên chỉ canh chữ cái tiếng Việt, và thế là chưa đủ: giao diện còn
/// vẽ mũi tên, dấu ngoặc kép nhọn, dấu ba chấm, ký hiệu cảnh báo. Đã dẫm phải
/// một lần — `▸` đứng đầu mỗi dòng nhật ký hiện ra thành ô vuông trống trên
/// Segoe UI, và chỉ lộ ra khi mở ứng dụng lên nhìn.
///
/// Nên thay vì liệt kê tay (rồi quên cập nhật), quét thẳng mã nguồn. Chuỗi nào
/// lọt vào giao diện thì cũng nằm trong mấy file này.
#[test]
fn moi_ky_hieu_giao_dien_deu_co_glyph() {
    let Some((duong_dan, byte)) =
        UNG_VIEN.iter().find_map(|p| std::fs::read(p).ok().map(|b| (*p, b)))
    else {
        eprintln!("bỏ qua: máy này không có phông nào trong danh sách");
        return;
    };
    let phong = FontRef::try_from_slice(&byte).expect("phông hỏng");

    let goc = env!("CARGO_MANIFEST_DIR");
    let mut thieu: Vec<String> = Vec::new();
    let mut da_xet = std::collections::BTreeSet::new();
    // Chỉ những file mà chuỗi trong đó đi thẳng vào egui. `bao_cao.rs` sinh HTML
    // cho trình duyệt vẽ bằng bộ phông khác hẳn, nên nó không thuộc đây.
    for f in ["src/main.rs", "src/nhat_ky.rs", "src/xu_ly.rs"] {
        let nguon = std::fs::read_to_string(format!("{goc}/{f}")).expect("không đọc được nguồn");
        for c in chuoi_trong_nguon(&nguon).chars() {
            if c.is_ascii() || !da_xet.insert(c) {
                continue;
            }
            if phong.glyph_id(c).0 == 0 {
                thieu.push(format!("{c} (U+{:04X}) trong {f}", c as u32));
            }
        }
    }
    assert!(thieu.is_empty(), "{duong_dan} thiếu glyph: {}", thieu.join(", "));
}

/// Gom nội dung mọi chuỗi ký tự trong một file nguồn Rust.
///
/// Quét chuỗi thay vì quét cả file, vì cả file thì dính luôn phần **comment** —
/// mà comment thì đầy ký hiệu được nhắc tới chính vì chúng vẽ không được. Bản
/// quét đầu tiên của bài kiểm này báo đỏ ở đúng đoạn ghi chú giải thích vì sao
/// không được dùng `▸`.
///
/// Bộ quét thô: theo dõi trạng thái trong/ngoài chuỗi, hiểu dấu thoát `\"` và
/// chuỗi thô `r"…"` / `r#"…"#`, bỏ qua comment một dòng. Đủ cho mã của chính dự
/// án này; nó không phải bộ phân tích cú pháp Rust.
fn chuoi_trong_nguon(nguon: &str) -> String {
    let b: Vec<char> = nguon.chars().collect();
    let mut ra = String::new();
    let mut i = 0usize;
    while i < b.len() {
        // Comment một dòng.
        if b[i] == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
            continue;
        }
        // Chuỗi thô: `r"…"`, `r#"…"#`, `r##"…"##`.
        if b[i] == 'r' && matches!(b.get(i + 1), Some('"') | Some('#')) {
            let mut rao = 0usize;
            let mut j = i + 1;
            while b.get(j) == Some(&'#') {
                rao += 1;
                j += 1;
            }
            if b.get(j) == Some(&'"') {
                j += 1;
                let dong: String = std::iter::once('"').chain(std::iter::repeat_n('#', rao)).collect();
                let con_lai: String = b[j..].iter().collect();
                let het = con_lai.find(&dong).map(|k| j + con_lai[..k].chars().count());
                let het = het.unwrap_or(b.len());
                ra.extend(&b[j..het]);
                i = het + 1 + rao;
                continue;
            }
        }
        // Chuỗi thường.
        if b[i] == '"' {
            i += 1;
            while i < b.len() && b[i] != '"' {
                if b[i] == '\\' {
                    i += 1;
                }
                if i < b.len() {
                    ra.push(b[i]);
                }
                i += 1;
            }
            i += 1;
            ra.push('\n');
            continue;
        }
        i += 1;
    }
    ra
}

#[test]
fn danh_sach_khop_ma_nguon() {
    // Bài kiểm trên chỉ có nghĩa nếu nó kiểm đúng cái phông mà ứng dụng nạp.
    // Chép tay hai bản danh sách thì sớm muộn chúng lệch nhau, nên đối chiếu
    // thẳng với mã nguồn.
    let nguon = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"))
        .expect("không đọc được main.rs");
    for p in UNG_VIEN {
        assert!(
            nguon.contains(p),
            "danh sách phông trong bài kiểm đã lệch với main.rs: thiếu {p}"
        );
    }
}
