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
///
/// **`đ` không đối xứng với `d`.** Thiếu dấu là chuyện thường — gõ `dang` khi
/// định gõ `đang` chỉ là quên một phím. Chiều ngược lại thì không: muốn ra `đ`
/// phải gõ hẳn `dd`, nên không ai vô tình gõ `đ` khi định gõ `d`. Xếp hai chiều
/// như nhau thì bộ sửa đề nghị đổi `đ` thành `d` — mà đó là hai phụ âm khác
/// nhau, đổi là hỏng nghĩa.
fn cung_nen(c: char) -> &'static [char] {
    match c {
        'a' | 'ă' | 'â' => &['a', 'ă', 'â'],
        'e' | 'ê' => &['e', 'ê'],
        'o' | 'ô' | 'ơ' => &['o', 'ô', 'ơ'],
        'u' | 'ư' => &['u', 'ư'],
        'd' => &['d', 'đ'],
        'đ' => &['đ'],
        _ => &[],
    }
}

/// Trần số ứng viên trả về.
///
/// Nới rộng khi thêm phép chèn và phép thay chữ: danh sách thô giờ lớn hơn
/// nhiều, nhưng tầng trên còn lọc tiếp bằng từ điển rồi mới dùng, nên cắt sớm ở
/// đây là cắt mất đúng ứng viên mà từ điển sẽ xác nhận.
const MAX_UNG_VIEN: usize = 40;

/// Sinh các cách sửa cho một tiếng, đã lọc và xếp hạng.
///
/// `tieng` đưa vào dạng nào thì trả về dạng ấy về mặt viết hoa.
pub fn sinh(tieng: &str) -> Vec<UngVien> {
    let thap = tieng.to_lowercase();
    let mut ra: Vec<UngVien> = Vec::new();

    // Thanh của bản gốc. Ứng viên giữ nguyên thanh gốc được ưu tiên: người ta
    // hiếm khi gõ nhầm dấu thanh mà đúng mọi thứ khác.
    //
    // Không thấy dấu nào thì thanh gốc là **thanh ngang**, không phải "không
    // có". Để `None` là hỏng lặng lẽ: mọi thanh đều bị tính là khác thanh gốc,
    // kể cả thanh ngang, nên `đnag` cho ra `đang`, `đàng`, `đáng`, `đãng` hoà
    // giá nhau hết — trong khi `đang` mới là bản giữ nguyên đúng thứ người ta gõ.
    let thanh_goc = Some(
        thap.chars()
            .map(|c| am_tiet::bo_thanh(c).1)
            .find(|&t| t != am_tiet::NGANG)
            .unwrap_or(am_tiet::NGANG),
    );

    // Bộ 1 — đổi dấu phụ, giữ nguyên bộ khung chữ cái.
    let khong_thanh: String = thap.chars().map(|c| am_tiet::bo_thanh(c).0).collect();
    for bt in doi_dau_phu(&khong_thanh) {
        let gia_nen = khac_bao_nhieu(&khong_thanh, &bt);
        them_moi_thanh(&bt, thanh_goc, gia_nen * 2, &mut ra);
    }

    // Bộ 2 — thêm/bớt/đảo/thay một chữ cái. Đắt hơn bộ 1 vì đây là gõ trượt
    // phím, hiếm hơn là quên dấu.
    for (bt, gia_sua) in sua_mot_chu(&khong_thanh) {
        them_moi_thanh(&bt, thanh_goc, gia_sua, &mut ra);
        // Đổi dấu phụ *sau khi* thêm bớt chữ: `nguoiw` kiểu gõ hỏng cần cả hai.
        for bt2 in doi_dau_phu(&bt) {
            let g = gia_sua + khac_bao_nhieu(&bt, &bt2) * 2;
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

/// Chữ cái dùng để chèn và thay thế.
///
/// Chỉ chữ cái ASCII cộng `đ`: phần dấu phụ do [`doi_dau_phu`] lo, nên chèn
/// thẳng `ư` hay `ô` ở đây là làm hai lần một việc.
const CHU_CAI: [char; 27] = [
    'a', 'b', 'c', 'd', 'đ', 'e', 'g', 'h', 'i', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't',
    'u', 'v', 'x', 'y', 'f', 'j', 'w', 'z',
];

/// Sửa một chữ: xoá, đảo, chèn, hoặc thay.
///
/// Bản đầu **không chèn và không thay**, vì mỗi vị trí 27 khả năng thì một tiếng
/// bốn chữ đẻ ra hơn hai trăm chuỗi. Cái giá của việc bỏ qua ấy đo được trên
/// sách thật: `chúg` cần chèn `n` để thành `chúng`, `kông` cần chèn `h`,
/// `khônh` cần thay `h` bằng `g`, `bứac` cần thay `a` bằng `o`. Không sinh ra
/// thì tầng chọn dưới không có gì đúng để chọn, và nó chọn `chừ`.
///
/// Sinh ra được vì mọi ứng viên đều bị lọc hai lần: [`sinh`] bỏ chuỗi không
/// ghép thành tiếng hợp lệ, rồi tầng trên bỏ tiếp những tiếng không có trong từ
/// điển. Cái còn lại chỉ vài chục.
fn sua_mot_chu(khung: &str) -> Vec<(String, u32)> {
    /// Giá gốc của một phép sửa một chữ.
    const GIA: u32 = 6;
    /// Giá của phép **đảo hai chữ liền nhau**, rẻ hơn mọi phép khác.
    ///
    /// Đảo chữ giữ nguyên **từng ký tự** người ta đã gõ, chỉ đổi thứ tự; xoá
    /// hay chèn thì thêm bớt hẳn một chữ. Cùng lý do tách chữ dính được xét
    /// trước sửa chữ: cách nào giữ được nhiều thứ người viết đã gõ hơn thì gần
    /// bản gốc hơn.
    ///
    /// Không phân ra thì `khôgn` cho `khôn` (xoá `g`) và `không` (đảo `gn`)
    /// cùng giá, rồi `khôn` thắng vì đứng trước trong bảng chữ cái.
    const GIA_DAO: u32 = 4;
    /// Phụ thu khi phép thay đụng vào **nguyên âm**.
    ///
    /// Đổi phụ âm cuối là lỗi phổ biến bậc nhất của tiếng Việt — lẫn `n` với
    /// `ng`, `nh` với `ng`, `c` với `t`. Đổi nguyên âm giữa từ thì hiếm hơn
    /// nhiều, vì nguyên âm mới là chỗ người viết nhớ rõ nhất.
    ///
    /// Không phân hai loại thì chúng hoà giá, và ai thắng là do thứ tự bảng chữ
    /// cái: `đing` cho ra `đinh` và `đang` cùng giá, rồi `đang` thắng vì `a`
    /// đứng trước `i`.
    const PHU_THU_NGUYEN_AM: u32 = 2;

    let ky_tu: Vec<char> = khung.chars().collect();
    let mut ra = Vec::new();
    for i in 0..ky_tu.len() {
        // Xoá chữ thứ i — bắt lỗi giữ phím: `khôngg`, `nhưnng`.
        let mut m = ky_tu.clone();
        m.remove(i);
        if m.len() >= 2 {
            ra.push((m.into_iter().collect(), GIA));
        }
        // Đảo chữ i với i+1 — bắt lỗi gõ nhanh: `nhưgn`, `khôgn`, `độgn`, `đnag`.
        if i + 1 < ky_tu.len() {
            let mut m = ky_tu.clone();
            m.swap(i, i + 1);
            ra.push((m.into_iter().collect(), GIA_DAO));
        }
        // Thay chữ thứ i — bắt lỗi trượt phím: `khônh` → `không`.
        for &c in CHU_CAI.iter() {
            if c == ky_tu[i] {
                continue;
            }
            // Không bao giờ đổi `đ` thành `d`: hai phụ âm khác nhau, và không ai
            // vô tình gõ `đ` (phải gõ `dd`) khi định gõ `d`. Xem [`cung_nen`].
            if ky_tu[i] == 'đ' && c == 'd' {
                continue;
            }
            let mut m = ky_tu.clone();
            m[i] = c;
            let gia = GIA
                + if am_tiet::la_nguyen_am(ky_tu[i]) || am_tiet::la_nguyen_am(c) {
                    PHU_THU_NGUYEN_AM
                } else {
                    0
                };
            ra.push((m.into_iter().collect(), gia));
        }
    }
    // Chèn một chữ vào mọi vị trí, kể cả đầu và cuối — bắt lỗi hụt phím:
    // `chúg` → `chúng`, `kông` → `không`.
    for i in 0..=ky_tu.len() {
        for &c in CHU_CAI.iter() {
            let mut m = ky_tu.clone();
            m.insert(i, c);
            ra.push((m.into_iter().collect(), GIA));
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

    /// Ứng viên đứng đầu bảng — cái mà bộ sửa chọn khi không có bằng chứng nào khác.
    fn dau_bang(tieng: &str) -> String {
        sinh(tieng).first().map(|u| u.chu.clone()).unwrap_or_default()
    }

    #[test]
    fn chu_khong_dau_thi_uu_tien_ung_vien_khong_dau() {
        // `đnag` là `đang` gõ đảo hai chữ. Bản đầu để "thanh gốc" là *không có*
        // thay vì *thanh ngang*, nên mọi thanh đều bị tính là khác thanh gốc —
        // `đang`, `đàng`, `đáng`, `đãng` hoà giá nhau hết, rồi ai thắng là do
        // thứ tự bảng chữ cái.
        assert_eq!(dau_bang("đnag"), "đang");
        assert_eq!(dau_bang("khôgn"), "không");
    }

    #[test]
    fn doi_phu_am_re_hon_doi_nguyen_am() {
        // `đing` → `đinh` (đổi phụ âm cuối) phải rẻ hơn `đang` (đổi nguyên âm).
        // Lẫn phụ âm cuối `n`/`ng`/`nh` là lỗi phổ biến bậc nhất của tiếng
        // Việt; đổi nguyên âm giữa từ thì hiếm hơn nhiều.
        let uv = sinh("đing");
        let gia = |c: &str| uv.iter().find(|u| u.chu == c).map(|u| u.gia);
        assert!(gia("đinh") < gia("đang"), "{uv:?}");
    }

    #[test]
    fn khong_bao_gio_doi_d_gach_thanh_d_thuong() {
        // `đ` và `d` là hai phụ âm khác nhau. Thiếu dấu là chuyện thường (`dang`
        // khi định gõ `đang`), nhưng chiều ngược lại thì không: muốn ra `đ` phải
        // gõ hẳn `dd`.
        assert!(
            sinh("đang").iter().all(|u| !u.chu.starts_with('d')),
            "{:?}",
            sinh("đang").iter().filter(|u| u.chu.starts_with('d')).collect::<Vec<_>>()
        );
        // Chiều đúng vẫn phải còn.
        assert!(sinh("dang").iter().any(|u| u.chu == "đang"));
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
        // Chữ **đầu** phải hoa. Không đòi nó là chữ cái nào: phép sinh có thay
        // chữ nên ứng viên đổi cả âm đầu là chuyện bình thường.
        assert!(sinh("Thuơng")
            .iter()
            .all(|u| u.chu.chars().next().is_some_and(|c| c.is_uppercase())));
        assert!(sinh("thuơng")
            .iter()
            .all(|u| u.chu.chars().next().is_some_and(|c| c.is_lowercase())));
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
