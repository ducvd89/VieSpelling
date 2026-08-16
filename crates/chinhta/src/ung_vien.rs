//! Từ một tiếng sai, sinh ra các cách sửa có lý — và **chỉ** những cách có lý.
//!
//! Phép sinh ứng viên kiểu chung (khoảng cách sửa 1 trên bảng chữ cái) không
//! dùng được ở đây: nó đẻ ra hàng trăm chuỗi mà gần hết không phải tiếng Việt,
//! rồi đẩy hết gánh nặng sang tầng chấm điểm. Ở đây đi ngược lại — sinh **theo
//! cách người ta gõ sai**, rồi lọc bằng chính bộ kiểm cấu tạo âm tiết, nên mọi
//! ứng viên trả về đều là tiếng Việt viết đúng.
//!
//! Ba kiểu gõ sai chiếm gần hết số lỗi trong ebook:
//!
//! | Kiểu | Ví dụ | Sinh bằng |
//! |---|---|---|
//! | Rơi hoặc lạc dấu phụ | `thuơng` → `thương` | đổi `o/ô/ơ`, `u/ư`, `a/ă/â`, `e/ê`, `d/đ` |
//! | Rơi hoặc sai dấu thanh | `hoi` → `hỏi` | thử cả sáu thanh |
//! | Thừa/thiếu/đảo một chữ | `khôngg` → `không` | xoá, đảo, chèn một chữ |
//!
//! Xếp hạng theo **số chỗ phải đổi**, ít nhất lên trước. Xếp hạng chỉ để cắt
//! bớt danh sách; ai được chọn thì tầng mô hình ngôn ngữ quyết, vì `chia sẻ` và
//! `chia xẻ` cùng cách bản gốc một chỗ đổi mà chỉ ngữ cảnh mới phân được.

use crate::am_tiet::{self, AmTiet};

/// Một cách sửa, kèm giá.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UngVien {
    pub chu: String,
    /// Càng nhỏ càng giống bản gốc. Dùng để cắt danh sách, không phải để chọn.
    pub gia: u32,
}

/// Các nguyên âm cùng một chữ cái nền — đây là nhóm mà người gõ hay lẫn, vì
/// trên bàn phím Telex chúng chỉ khác nhau một phím phụ.
fn cung_nen(c: char) -> &'static [char] {
    match c {
        'a' | 'ă' | 'â' => &['a', 'ă', 'â'],
        'e' | 'ê' => &['e', 'ê'],
        'o' | 'ô' | 'ơ' => &['o', 'ô', 'ơ'],
        'u' | 'ư' => &['u', 'ư'],
        'd' | 'đ' => &['d', 'đ'],
        _ => &[],
    }
}

const MAX_UNG_VIEN: usize = 12;

/// Sinh các cách sửa cho một tiếng, đã lọc và xếp hạng.
///
/// `tieng` đưa vào dạng nào thì trả về dạng ấy về mặt viết hoa.
pub fn sinh(tieng: &str) -> Vec<UngVien> {
    let thap = tieng.to_lowercase();
    let mut ra: Vec<UngVien> = Vec::new();

    // Thanh của bản gốc, nếu có. Ứng viên giữ nguyên thanh gốc được ưu tiên:
    // người ta hiếm khi gõ nhầm dấu thanh mà đúng mọi thứ khác.
    let thanh_goc = thap.chars().map(|c| am_tiet::bo_thanh(c).1).find(|&t| t != am_tiet::NGANG);

    // Bộ 1 — đổi dấu phụ, giữ nguyên bộ khung chữ cái.
    let khong_thanh: String = thap.chars().map(|c| am_tiet::bo_thanh(c).0).collect();
    for bt in doi_dau_phu(&khong_thanh) {
        let gia_nen = khac_bao_nhieu(&khong_thanh, &bt);
        them_moi_thanh(&bt, thanh_goc, gia_nen * 2, &mut ra);
    }

    // Bộ 2 — thêm/bớt/đảo một chữ cái. Đắt hơn bộ 1 vì đây là gõ trượt phím,
    // hiếm hơn là quên dấu.
    for bt in sua_mot_chu(&khong_thanh) {
        them_moi_thanh(&bt, thanh_goc, 6, &mut ra);
        // Đổi dấu phụ *sau khi* thêm bớt chữ: `nguoiw` kiểu gõ hỏng cần cả hai.
        for bt2 in doi_dau_phu(&bt) {
            let g = 6 + khac_bao_nhieu(&bt, &bt2) * 2;
            them_moi_thanh(&bt2, thanh_goc, g, &mut ra);
        }
    }

    ra.sort_by(|a, b| a.gia.cmp(&b.gia).then_with(|| a.chu.cmp(&b.chu)));
    ra.dedup_by(|a, b| a.chu == b.chu);
    ra.retain(|u| u.chu != thap);
    ra.truncate(MAX_UNG_VIEN);

    // Trả lại kiểu viết hoa của bản gốc.
    if tieng.chars().next().is_some_and(|c| c.is_uppercase()) {
        for u in ra.iter_mut() {
            let mut c = u.chu.chars();
            if let Some(d) = c.next() {
                u.chu = d.to_uppercase().collect::<String>() + c.as_str();
            }
        }
    }
    ra
}

/// Với một khung chữ không thanh, thử gắn từng thanh rồi giữ lại cái nào ghép
/// thành tiếng hợp lệ.
fn them_moi_thanh(khung: &str, thanh_goc: Option<u8>, gia_nen: u32, ra: &mut Vec<UngVien>) {
    // Chẻ khung bằng `tach_khung` chứ **không** bằng `tach`: khung ở đây chưa
    // mang dấu thanh, mà `tach` loại thẳng vần khép không dấu.
    let Some((am_dau, van)) = am_tiet::tach_khung(khung) else { return };
    for thanh in 0u8..6 {
        if !am_tiet::thanh_hop_le(&van, thanh) {
            continue;
        }
        let at = AmTiet {
            am_dau: am_dau.clone(),
            van: van.clone(),
            thanh,
            hoa_dau: false,
            hoa_het: false,
        };
        let chu = am_tiet::ghep(&at, false);
        if am_tiet::tach(&chu).is_none() {
            continue;
        }
        let gia = gia_nen + if Some(thanh) == thanh_goc { 0 } else { 3 };
        ra.push(UngVien { chu, gia });
    }
}

/// Mọi tổ hợp đổi dấu phụ của một khung chữ.
///
/// Chặn ở 3 vị trí đổi được để khỏi bùng nổ tổ hợp — âm tiết tiếng Việt dài
/// nhất cũng chỉ có 3 nguyên âm, nên chặn này không cắt mất ứng viên thật nào.
fn doi_dau_phu(khung: &str) -> Vec<String> {
    let ky_tu: Vec<char> = khung.chars().collect();
    let vi_tri: Vec<usize> = (0..ky_tu.len()).filter(|&i| !cung_nen(ky_tu[i]).is_empty()).collect();
    if vi_tri.len() > 3 {
        return vec![khung.to_string()];
    }
    let mut ra = vec![ky_tu.clone()];
    for &i in &vi_tri {
        let mut tiep = Vec::with_capacity(ra.len() * 3);
        for nen in ra {
            for &thay in cung_nen(ky_tu[i]) {
                let mut m = nen.clone();
                m[i] = thay;
                tiep.push(m);
            }
        }
        ra = tiep;
    }
    ra.into_iter().map(|v| v.into_iter().collect()).collect()
}

/// Xoá một chữ, đảo hai chữ liền nhau, hoặc nhân đôi một chữ.
///
/// Không chèn chữ tuỳ ý: chèn thì mỗi vị trí 29 khả năng, nhân với độ dài từ ra
/// hàng trăm ứng viên, mà lỗi thiếu hẳn một chữ cái thì hiếm hơn nhiều so với
/// thừa chữ (giữ phím) hay đảo chữ (gõ nhanh).
fn sua_mot_chu(khung: &str) -> Vec<String> {
    let ky_tu: Vec<char> = khung.chars().collect();
    let mut ra = Vec::new();
    for i in 0..ky_tu.len() {
        // Xoá chữ thứ i — bắt lỗi giữ phím: `khôngg`, `nhưnng`.
        let mut m = ky_tu.clone();
        m.remove(i);
        if m.len() >= 2 {
            ra.push(m.into_iter().collect());
        }
        // Đảo chữ i với i+1 — bắt lỗi gõ nhanh: `nhưng` thành `nhưnq`… đúng hơn
        // là `hoạt` thành `hoäta`.
        if i + 1 < ky_tu.len() {
            let mut m = ky_tu.clone();
            m.swap(i, i + 1);
            ra.push(m.into_iter().collect());
        }
    }
    ra
}

fn khac_bao_nhieu(a: &str, b: &str) -> u32 {
    a.chars().zip(b.chars()).filter(|(x, y)| x != y).count() as u32
        + (a.chars().count() as i64 - b.chars().count() as i64).unsigned_abs() as u32
}

#[cfg(test)]
mod kiem {
    use super::*;

    fn co(tieng: &str, mong: &str) -> bool {
        sinh(tieng).iter().any(|u| u.chu == mong)
    }

    #[test]
    fn sua_loi_lac_dau_phu() {
        // Lỗi số một trong ebook tiếng Việt: gõ `uo` rồi đặt dấu móc nhầm chữ.
        assert!(co("thuơng", "thương"), "{:?}", sinh("thuơng"));
        assert!(co("nguơi", "người"), "{:?}", sinh("nguơi"));
    }

    #[test]
    fn sua_loi_giu_phim() {
        assert!(co("khôngg", "không"), "{:?}", sinh("khôngg"));
    }

    #[test]
    fn khong_de_ra_chuoi_khong_phai_tieng() {
        // Mọi ứng viên phải tự nó là một tiếng viết đúng — nếu không thì ta chỉ
        // đang đổi một lỗi lấy một lỗi khác.
        for tieng in ["thuơng", "khôngg", "nguơi", "xxyyzz"] {
            for u in sinh(tieng) {
                assert!(am_tiet::hop_le(&u.chu), "ứng viên không hợp lệ: {}", u.chu);
            }
        }
    }

    #[test]
    fn sinh_duoc_cho_van_khep() {
        // Vần khép (`hoat`, `biet`, `mot`) từng không sinh nổi ứng viên nào vì
        // phần này chẻ khung bằng `tach`, mà `tach` loại vần khép không dấu.
        // Lỗi im lặng: không báo gì, chỉ là mọi tiếng kết thúc bằng p/t/c/ch
        // đều không bao giờ được sửa.
        assert!(co("hoat", "hoạt"), "{:?}", sinh("hoat"));
        assert!(co("biet", "biết"), "{:?}", sinh("biet"));
        assert!(co("mot", "một"), "{:?}", sinh("mot"));
    }

    #[test]
    fn giu_kieu_viet_hoa() {
        assert!(sinh("Thuơng").iter().all(|u| u.chu.starts_with('T')));
    }

    #[test]
    fn ung_vien_giu_thanh_goc_duoc_uu_tien() {
        // `thuơng` không mang dấu thanh nên `thương` (cũng không dấu) phải đứng
        // trên `thường`, `thưởng`… Người gõ trượt dấu móc, không trượt dấu thanh.
        let uv = sinh("thuơng");
        let vt = |c: &str| uv.iter().position(|u| u.chu == c);
        assert!(vt("thương") < vt("thường"), "{uv:?}");
    }
}
