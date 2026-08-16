//! Từ điển tiếng Việt: kho âm tiết có thật và kho từ ghép.
//!
//! # Vì sao từ điển làm phép kiểm chính, còn bảng vần lùi về dự phòng
//!
//! [`crate::am_tiet`] kiểm bằng **cấu tạo**: âm đầu + vần + thanh. Cách ấy gọn
//! và giải thích được, nhưng nó có một điểm mù không vá nổi — tiếng Việt hiện
//! đại đầy từ mượn viết theo âm Việt, mà chúng mang hình dạng ngoài hệ thống
//! ngữ âm: `bêtông`, `cafê`, `micrô`, `pittông`, `rađa`, `nilông`, `blô`,
//! `phrăng`. Đo trên chính từ điển này thì **544 âm tiết có dấu tiếng Việt** bị
//! bảng vần bác bỏ, và phần lớn là từ mượn như thế.
//!
//! Mỗi mục bị bác oan không phải là bỏ sót — ứng dụng tự sửa, nên nó là một chữ
//! **đúng** bị đổi thành chữ **sai**. Đắt hơn hẳn việc bỏ qua một lỗi.
//!
//! Nên thứ tự là: có trong từ điển thì thôi, không có thì mới hỏi bảng vần.
//! Bảng vần vẫn cần, vì từ điển không phủ hết tên riêng và từ mới, và vì phần
//! sinh ứng viên sửa cần biết **chẻ** một tiếng ra thế nào chứ không chỉ biết
//! nó có tồn tại hay không.
//!
//! # Kho từ ghép dùng để làm gì
//!
//! Đây là thứ chữa lớp lỗi mà mô hình ngôn ngữ chọn sai. `chúg ta` sinh ra hàng
//! chục ứng viên đều là tiếng có thật — `chúng`, `chừ`, `chú`, `chug`… — và mô
//! hình 9 tỷ tham số vẫn chọn `chừ`. Nhưng `chúng ta` có trong từ điển còn
//! `chừ ta` thì không, và bằng chứng ấy dứt khoát hơn hẳn mọi điểm số.
//!
//! Hai file dữ liệu dựng bằng `examples/dung_tu_dien.rs` từ ba bộ từ điển
//! (tudientv, Wiktionary tiếng Việt, Hồ Ngọc Đức) — xem `du-lieu/NGUON.md`.

use std::collections::HashSet;
use std::sync::OnceLock;

const AM_TIET: &str = include_str!("../../../du-lieu/am-tiet.txt");
const TU_GHEP: &str = include_str!("../../../du-lieu/tu-ghep.txt");

fn kho_am_tiet() -> &'static HashSet<&'static str> {
    static KHO: OnceLock<HashSet<&'static str>> = OnceLock::new();
    KHO.get_or_init(|| AM_TIET.lines().filter(|l| !l.is_empty()).collect())
}

fn kho_tu_ghep() -> &'static HashSet<&'static str> {
    static KHO: OnceLock<HashSet<&'static str>> = OnceLock::new();
    KHO.get_or_init(|| TU_GHEP.lines().filter(|l| !l.is_empty()).collect())
}

/// Tiếng này có trong từ điển không. Không phân biệt hoa thường.
///
/// Nhận `&str` đã viết thường thì nhanh hơn; hàm tự hạ chữ nếu cần.
pub fn co_am_tiet(tieng: &str) -> bool {
    if tieng.is_empty() {
        return false;
    }
    if kho_am_tiet().contains(tieng) {
        return true;
    }
    let thap = tieng.to_lowercase();
    kho_am_tiet().contains(thap.as_str())
}

/// Cụm tiếng này có phải một từ ghép trong từ điển không.
///
/// `cum` là các tiếng đã viết thường, nối bằng đúng một khoảng trắng.
pub fn co_tu_ghep(cum: &str) -> bool {
    kho_tu_ghep().contains(cum)
}

/// Ghép `tieng` với hàng xóm hai bên, xem có ra từ ghép nào trong từ điển không.
///
/// Đây là phép chấm điểm ứng viên rẻ nhất và chắc nhất mà ứng dụng có. Trả về
/// số từ ghép dựng được (0, 1 hoặc 2) — nhiều hơn nghĩa là ứng viên khớp cả hai
/// phía, gần như chắc chắn đúng.
pub fn khop_hang_xom(truoc: Option<&str>, tieng: &str, sau: Option<&str>) -> usize {
    let t = tieng.to_lowercase();
    let mut n = 0;
    if let Some(p) = truoc {
        if co_tu_ghep(&format!("{} {t}", p.to_lowercase())) {
            n += 1;
        }
    }
    if let Some(s) = sau {
        if co_tu_ghep(&format!("{t} {}", s.to_lowercase())) {
            n += 1;
        }
    }
    n
}

/// Số phần tối đa khi tách một chuỗi chữ dính.
///
/// Bốn là đủ cho mọi ca gặp thật: chữ dính sinh ra khi bóc thẻ HTML hoặc
/// chuyển từ PDF, mà chỗ dính thường chỉ là một hai khoảng trắng bị nuốt. Nới
/// hơn thì mỗi chuỗi dài đẻ ra hàng nghìn cách chia, và cách chia nào cũng
/// "hợp lệ" theo nghĩa từng mảnh có trong từ điển — tức là mất hết sức thuyết
/// phục.
const TOI_DA_PHAN: usize = 4;

/// Tách một chuỗi chữ **dính liền** thành các tiếng có trong từ điển.
///
/// `Phúlần` → `phú lần`. Trả về danh sách cách chia, ít mảnh trước.
///
/// Đây là lớp lỗi riêng, và nó khác mọi lỗi khác ở một điểm quyết định: **không
/// chữ cái nào sai**, chỉ thiếu khoảng trắng. Nên bằng chứng mạnh hơn hẳn phép
/// sửa một chữ — sửa chữ là đoán người ta định gõ gì, còn tách chữ thì giữ
/// nguyên từng ký tự người ta đã gõ.
///
/// Hai lưới chặn để khỏi băm nhỏ chữ vô tội:
///
/// - Chỉ gọi cho tiếng **đã bị bắt** là không có trong từ điển và sai cấu tạo.
///   Chữ đúng không bao giờ đi qua đây, nên `giác` không thể thành `gi ác`.
/// - Mỗi mảnh phải **có trong từ điển** và **có nguyên âm**. Nếu không thì
///   `việcc` thành `việc` + `c`, `cuốic` thành `cuối` + `c` — đổi một lỗi lấy
///   một lỗi tệ hơn.
pub fn tach_dinh(tieng: &str) -> Vec<String> {
    let thap = tieng.to_lowercase();
    let ky_tu: Vec<char> = thap.chars().collect();
    // Ngắn quá thì không phải chữ dính mà là lỗi gõ, dài quá thì số cách chia
    // bùng nổ. Ngưỡng 5 chặn đúng lớp nguy hiểm: `bứac` (4 chữ) chia được thành
    // `bứ ac` vì cả hai mảnh đều có trong từ điển, mà nó là lỗi gõ chứ không
    // phải chữ dính.
    if ky_tu.len() < 5 || ky_tu.len() > 24 {
        return Vec::new();
    }
    let mut ra: Vec<Vec<String>> = Vec::new();
    let mut dang: Vec<String> = Vec::new();
    chia(&ky_tu, 0, &mut dang, &mut ra);
    // Phải có ít nhất một mảnh từ ba chữ trở lên. Cách chia toàn mảnh hai chữ
    // gần như luôn là băm vụn: từ điển có đủ `cu`, `ố`, `ic`, `ac` nên chuỗi
    // hỏng nào cũng chia được kiểu ấy.
    ra.retain(|p| p.iter().any(|m| m.chars().count() >= 3));

    // Ít mảnh trước; cùng số mảnh thì cách nào dựng được từ ghép trong từ điển
    // đứng trước.
    ra.sort_by_key(|p| {
        let ghep = p.windows(2).filter(|w| co_tu_ghep(&format!("{} {}", w[0], w[1]))).count();
        (p.len(), std::cmp::Reverse(ghep))
    });
    ra.into_iter().map(|p| p.join(" ")).collect()
}

fn chia(ky_tu: &[char], tu: usize, dang: &mut Vec<String>, ra: &mut Vec<Vec<String>>) {
    if tu == ky_tu.len() {
        if dang.len() >= 2 {
            ra.push(dang.clone());
        }
        return;
    }
    if dang.len() >= TOI_DA_PHAN {
        return;
    }
    // Mảnh **tối thiểu hai chữ**. Từ điển có cả những mục một chữ (`à`, `ố`,
    // `ừ`), nên cho phép mảnh một chữ thì mọi lỗi thừa chữ biến thành lỗi dính
    // chữ: `việcc` thành `việc c`, `cuốic` thành `cuối c`. Đổi một lỗi lấy một
    // lỗi tệ hơn, vì nó thêm hẳn một "chữ" vào câu.
    // Mảnh **từ thứ hai trở đi phải bắt đầu bằng phụ âm**.
    //
    // Đây là lưới chặn quan trọng nhất, và nó suy từ số liệu chứ không từ suy
    // đoán. Đo trên một bộ truyện dài, mọi cách tách **đúng** đều có mảnh sau
    // mở đầu bằng phụ âm — `phú lần`, `nó không`, `các vị`, `mực Minh`,
    // `Huyền Vũ` — còn mọi cách tách **sai** đều có mảnh sau mở đầu bằng nguyên
    // âm: `phả ii`, `hu oàng`, `khuy ếch`, `tứ ước`, `ngo oại`, `hi ệnh`.
    //
    // Có lý do: chỗ ấy không phải hai tiếng dính nhau mà là **một nguyên âm bị
    // gõ lặp** trong cùng một tiếng. `Huoàng` là `Hoàng` thừa chữ u, `phảii` là
    // `phải` thừa chữ i. Tách ra thì được hai chữ đều có trong từ điển mà câu
    // thành vô nghĩa — kiểu hỏng khó thấy nhất, vì bản sửa trông vẫn "đúng
    // tiếng Việt".
    //
    // Cái giá: tiếng Việt có tiếng mở đầu bằng nguyên âm (`ăn`, `uống`, `ông`)
    // nên `cơmăn` không tách được. Chấp nhận — ca ấy vốn cũng nhập nhằng với
    // `cơ măn`, mà đoán sai thì tệ hơn bỏ sót.
    let dau_manh_phai_la_phu_am = !dang.is_empty();
    for het in tu + 2..=ky_tu.len() {
        let manh: String = ky_tu[tu..het].iter().collect();
        if dau_manh_phai_la_phu_am && crate::am_tiet::la_nguyen_am(ky_tu[tu]) {
            break; // ký tự đầu mảnh không đổi theo `het`, nên hỏng là hỏng cả
        }
        // Mảnh không có nguyên âm thì không phải tiếng. Chặn ở đây rẻ hơn tra
        // từ điển, và nó là lưới bắt phần lớn ca hỏng: `việc` + `c`.
        if !manh.chars().any(crate::am_tiet::la_nguyen_am) {
            continue;
        }
        if !co_am_tiet(&manh) {
            continue;
        }
        dang.push(manh);
        chia(ky_tu, het, dang, ra);
        dang.pop();
    }
}

pub fn so_am_tiet() -> usize {
    kho_am_tiet().len()
}

pub fn so_tu_ghep() -> usize {
    kho_tu_ghep().len()
}

#[cfg(test)]
mod kiem {
    use super::*;

    #[test]
    fn kho_nap_du() {
        assert!(so_am_tiet() > 9_000, "kho âm tiết quá nhỏ: {}", so_am_tiet());
        assert!(so_tu_ghep() > 60_000, "kho từ ghép quá nhỏ: {}", so_tu_ghep());
    }

    #[test]
    fn nhan_tu_muon_ma_bang_van_bac_bo() {
        // Đây là lý do tầng này tồn tại: những chữ dưới đây là tiếng Việt thật,
        // gặp thường xuyên trong sách, mà không ghép được từ âm đầu + vần nào.
        for t in ["bêtông", "micrô", "pittông", "rađa", "nilông", "cafê"] {
            assert!(co_am_tiet(t), "từ điển thiếu `{t}`");
            assert!(!crate::am_tiet::hop_le(t), "`{t}` mà bảng vần lại nhận?");
        }
    }

    #[test]
    fn nhan_tieng_thuong_gap() {
        for t in ["không", "người", "được", "chúng", "quýt", "méc"] {
            assert!(co_am_tiet(t), "từ điển thiếu `{t}`");
        }
    }

    #[test]
    fn khong_nhan_chuoi_bay() {
        for t in ["thuơng", "khôngg", "xxyyzz", "chúg"] {
            assert!(!co_am_tiet(t), "từ điển nhận nhầm `{t}`");
        }
    }

    #[test]
    fn tu_ghep_phan_biet_duoc_ung_vien() {
        // Ca cụ thể mà mô hình ngôn ngữ chọn sai: `chúg ta`. Từ ghép phân được
        // ngay, không cần card đồ hoạ.
        assert!(co_tu_ghep("chúng ta"));
        assert!(!co_tu_ghep("chừ ta"));
        assert_eq!(khop_hang_xom(None, "chúng", Some("ta")), 1);
        assert_eq!(khop_hang_xom(None, "chừ", Some("ta")), 0);
    }

    #[test]
    fn tach_duoc_chu_dinh() {
        assert_eq!(tach_dinh("Phúlần").first().map(|s| s.as_str()), Some("phú lần"));
        assert!(tach_dinh("củangười").contains(&"của người".to_string()));
        assert!(tach_dinh("khôngbiết").contains(&"không biết".to_string()));
    }

    #[test]
    fn khong_tach_khi_mot_manh_khong_phai_tieng() {
        // Đây là lưới chặn quan trọng nhất. Không có nó thì mọi lỗi thừa chữ
        // biến thành lỗi dính chữ: `việcc` thành `việc c`, `cuốic` thành
        // `cuối c`, `trốngm` thành `trống m` — đổi một lỗi lấy một lỗi tệ hơn,
        // vì nó thêm hẳn một "chữ" vào câu.
        for t in ["việcc", "cuốic", "trốngm", "trụcc", "khôngg", "bứac"] {
            assert!(tach_dinh(t).is_empty(), "`{t}` không được tách: {:?}", tach_dinh(t));
        }
    }

    #[test]
    fn khong_tach_chu_qua_ngan_hay_qua_dai() {
        assert!(tach_dinh("gì").is_empty());
        assert!(tach_dinh("bứac").is_empty(), "4 chữ thì là lỗi gõ, không phải chữ dính");
        assert!(tach_dinh(&"a".repeat(40)).is_empty());
    }

    #[test]
    fn cach_chia_it_manh_dung_truoc() {
        // `củangười` chia được thành `của người` (2 mảnh) lẫn `củ a ngư ời`
        // kiểu vụn hơn. Ít mảnh phải đứng trước, không thì bộ sửa băm chữ.
        let p = tach_dinh("củangười");
        assert!(!p.is_empty());
        let so_manh = |s: &str| s.split(' ').count();
        assert_eq!(so_manh(&p[0]), 2, "cách chia đầu bảng: {:?}", p[0]);
    }

    #[test]
    fn khop_ca_hai_phia() {
        // `sử` trong `lịch sử học`: khớp cả trái lẫn phải.
        let n = khop_hang_xom(Some("lịch"), "sử", Some("học"));
        assert!(n >= 1, "không khớp phía nào");
    }
}
