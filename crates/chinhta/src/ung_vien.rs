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
//! | Thừa/thiếu/đảo một chữ | `khôgn` → `không` | xoá, đảo, chèn một chữ |
//! | Bấm hai lần một phím | `khôngg` → `không` | bỏ một ký tự lặp |
//!
//! Xếp hạng theo **giá của phép sửa**, rẻ nhất lên trước, và giá đo bằng "giữ
//! được bao nhiêu thứ người ta đã gõ". Xếp hạng chỉ để cắt bớt danh sách; ai
//! được chọn thì tầng mô hình ngôn ngữ quyết, vì `chia sẻ` và `chia xẻ` cùng
//! cách bản gốc một chỗ đổi mà chỉ ngữ cảnh mới phân được.
//!
//! Bảng giá, rẻ tới đắt: **bỏ một ký tự lặp** (1) < **thêm** dấu phụ vào chữ trơn
//! (2) < **đụng vào** chữ đã mang dấu phụ (3) < đảo hai chữ liền nhau (4) <
//! xoá/chèn/thay một chữ (6) < thay nguyên âm (8). Cộng 1 nếu số chữ đổi. Mọi mức
//! đều suy từ một câu hỏi: người gõ phải bấm thêm hay bấm bớt bao nhiêu phím để ra
//! được chuỗi sai ấy.

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
///
/// Mấy nguyên âm riêng (`ă â ê ô ơ ư`) thì bất đối xứng **về giá** chứ không bị
/// chặn hẳn — xem [`GIA_DUNG_DAU_PHU`].
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

/// Chữ cái nền của một chữ cái riêng tiếng Việt: `ă â` → `a`, `ê` → `e`,
/// `ô ơ` → `o`, `ư` → `u`, `đ` → `d`. Chữ khác trả `None`.
fn nen_cua(c: char) -> Option<char> {
    Some(match c {
        'ă' | 'â' => 'a',
        'ê' => 'e',
        'ô' | 'ơ' => 'o',
        'ư' => 'u',
        'đ' => 'd',
        _ => return None,
    })
}

/// Giá của một vị trí **thêm** dấu phụ vào chữ trơn: `o` thành `ô` hay `ơ`, `a`
/// thành `ă` hay `â`, `d` thành `đ`.
///
/// Rẻ nhất trong mọi phép sửa (trừ phép bỏ ký tự lặp), vì đây là lỗi thường gặp
/// nhất của tiếng Việt gõ máy: người ta **quên** phím dấu phụ. Quên `w` là `ư` ra
/// `u`, quên một chữ `o` là `ô` ra `o`.
const GIA_THEM_DAU_PHU: u32 = 2;

/// Giá của một vị trí **đụng vào chữ đã mang dấu phụ** — đổi `ô` thành `o` (mất
/// hẳn) hay thành `ơ` (sang chữ khác cùng nhóm) đều tính như nhau.
///
/// Đắt hơn chiều thêm vào, và đây là chỗ suy luận dễ trượt. Lập luận bàn phím chỉ
/// biện minh được **một** chiều: mỗi chữ này đều cần một phím mà chữ trơn không cần
/// (`ô` = `oo`, `ơ` = `ow`, `ă` = `aw`, `â` = `aa`, `ê` = `ee`, `đ` = `dd`, `ư` =
/// `w`), nên **quên** phím ấy thì thường, còn bấm thêm một phím không cần thì không.
///
/// Nó **không** nói `ă` nên thành `â` hơn là thành `a` — `ă` và `â` là hai chữ khác
/// nhau, không phải hai biến thể của nhau, nên đổi sang `â` cũng là đổi sang một chữ
/// khác y như đổi sang `a`. Xếp `ă` → `â` rẻ hơn `ă` → `a` là bịa ra một sự thiên vị
/// không có căn cứ nào.
///
/// Nên bất đối xứng nằm đúng ở chỗ nó có lý: **giữ nguyên chữ đã mang dấu phụ**
/// rẻ hơn đụng vào nó, còn đụng vào theo hướng nào cũng cùng giá. `Duơng` cho ra
/// `Dương` (chỉ thêm dấu vào `u`, để `ơ` yên — giá 2) đứng trên `Duông` (đụng vào
/// `ơ` — giá 3), mà `măy` thì `may` với `mây` hoà giá, đúng như phải thế: chọn
/// giữa hai chữ khác nhau là việc của tầng từ ghép và tầng mô hình, không phải của
/// bảng giá này.
///
/// **Không chặn hẳn như `đ`.** Lớp lỗi "dấu móc rơi vào chữ bên cạnh" **cần** bỏ
/// dấu phụ mới sửa được: `thưộc` → `thuộc`, `xưống` → `xuống`, `cưồng` → `cuồng`,
/// `Đạơ` → `Đạo` — đo trên hai bộ truyện thì 22 phép sửa thuộc loại bỏ dấu phụ, và
/// quá nửa là loại này. Chặn hẳn thì mất cả.
///
/// **Và không được đắt hơn 3.** Từng đặt 5 — nghe hợp lý hơn, vì nó xếp "mất một
/// dấu phụ" nặng hơn "đảo hai chữ" (4). Nhưng đo ra thì hỏng, và hỏng ở một chỗ
/// không ai ngờ: `màn cẳu sổ` phải ra `màn cửa sổ`, mà `cửa` cần **cả** một phép
/// thêm dấu phụ lẫn một phép đụng vào dấu phụ nên giá nó lên 11 và rơi khỏi
/// [`MAX_UNG_VIEN`]. Tầng từ ghép chỉ chọn được trong danh sách nó nhận, nên cả
/// cuốn sách ra `màn của sổ` mà không tầng nào báo gì.
///
/// Nới [`MAX_UNG_VIEN`] lên cho vừa thì lại tệ hơn nữa — xem chú thích ở đó.
const GIA_DUNG_DAU_PHU: u32 = 3;

/// Hai ký tự có phải **cùng một chữ cái, chỉ khác dấu phụ hoặc dấu thanh** không.
///
/// `u`/`ư`, `o`/`ô`/`ơ`, `a`/`ă`/`â`, `e`/`ê`, và mọi dấu thanh trên chúng:
/// `ơ`/`ớ`/`ộ`/`o` đều là một chữ cái. Đây là nhóm **nguyên âm** mà người gõ lẫn
/// nhau, vì trên bàn phím chúng chỉ khác một phím phụ.
///
/// **`đ` không nằm trong nhóm ấy.** `đ` và `d` là hai **phụ âm** khác nhau — đó là
/// nguyên tắc có từ đầu repo, xem [`cung_nen`]. Chỗ này dễ lẫn vì cả hai chuyện đều
/// đúng cùng lúc: thêm dấu vào `d` để ra `đ` là phép sửa **rẻ** (giá 2, người ta
/// quên phím `dd` suốt), nhưng nó vẫn là **đổi phụ âm đầu** khi rơi vào đầu chữ.
/// Rẻ không có nghĩa là cùng chữ cái.
///
/// Gộp chúng lại thì `duợc` ra `được` — mà đo trên Phàm Nhân Tu Tiên thì cả hai chỗ
/// `duợc` đều là `dược`, và một trong hai chỗ có ngay `dược tính` viết đúng ở câu
/// bên cạnh. Tách ra thì `dược` thắng vì nó giữ `d`.
pub fn cung_chu_cai(a: char, b: char) -> bool {
    let nen = |c: char| {
        let thap = c.to_lowercase().next().unwrap_or(c);
        let (khong_thanh, _) = am_tiet::bo_thanh(thap);
        if khong_thanh == 'đ' {
            return 'đ';
        }
        nen_cua(khong_thanh).unwrap_or(khong_thanh)
    };
    nen(a) == nen(b)
}

/// Giá của phép **bỏ một ký tự lặp**: `khôngg` → `không`, `phảii` → `phải`,
/// `lưư` → `lưu`, `Hơnn` → `Hơn`.
///
/// Rẻ nhất bảng, và rẻ có căn cứ: **tiếng Việt gần như không có hai ký tự giống hệt
/// nhau đứng cạnh nhau.** Ngoại lệ duy nhất đáng kể là `oo` trong `xoong`, `boong`,
/// `coong`. Nên gặp `gg`, `ii`, `nn`, `tt` thì gần như chắc chắn là phím bị bấm hai
/// lần, chứ không phải chữ tác giả định viết — không cần cân nhắc cách sửa nào khác
/// trước nó.
///
/// Bằng 0 chứ không bằng 1 vì phép này **đổi số chữ**, nên [`sinh`] còn cộng thêm 1
/// cho nó. Tổng thành 1, đúng một bậc dưới phép rẻ thứ hai là thêm dấu phụ (2).
const GIA_BO_LAP: u32 = 0;

/// Mọi cách bỏ **một** ký tự trong một cặp ký tự lặp.
///
/// Hai nhóm được miễn, và cả hai đều có lý riêng:
///
/// - **`oo`** là chữ thật: `xoong`, `boong`, `coong`.
/// - **Chữ mang dấu phụ** (`ưư`, `ơơ`, `ôô`, `ăă`, `ââ`, `êê`, `đđ`) thì để tầng
///   dấu phụ lo, vì ở đó cặp lặp còn một cách giải thích thứ hai và cách ấy mới
///   đúng: **chữ trơn bên cạnh bị ăn dấu theo**. `ưu` là vần rất phổ biến — `lưu`,
///   `hưu`, `mưu`, `bưu`, `cứu` — nên `lưư` gần như chắc chắn là `lưu` gõ trượt, chứ
///   không phải `lư` bấm hai lần. Đo trên sách thật cũng ra `lưu`. Xếp phép bỏ lặp
///   rẻ hơn thì `lư` thắng (giá 1 so với 3), và thắng sai.
fn bo_ky_tu_lap(khung: &str) -> Vec<String> {
    let ky_tu: Vec<char> = khung.chars().collect();
    let mut ra = Vec::new();
    for i in 0..ky_tu.len().saturating_sub(1) {
        if ky_tu[i] != ky_tu[i + 1] || ky_tu[i] == 'o' || nen_cua(ky_tu[i]).is_some() {
            continue;
        }
        let mut m = ky_tu.clone();
        m.remove(i);
        if m.len() >= 2 {
            ra.push(m.into_iter().collect());
        }
    }
    ra
}

/// Ứng viên **chỉ thêm đúng một dấu phụ**, không đổi gì khác — kể cả dấu thanh.
///
/// Nhận ra bằng giá, và chỉ có **một** đường sinh ra giá ấy: [`GIA_THEM_DAU_PHU`]
/// ở đúng một vị trí, thanh giữ nguyên (không cộng 3), số chữ không đổi (không
/// cộng 1). Phép sửa chữ rẻ nhất là đảo hai chữ liền nhau, giá 4, nên không đường
/// nào khác chạm tới 2 được.
///
/// Tầng trên dùng nó để nhận ra phép sửa **cơ học**: `Duơng` → `Dương`, `thuơng`
/// → `thương`, `Nguời` → `Người`. Ở đó không có gì để đoán, nên cũng không có gì
/// để hỏi mô hình — mà hỏi thì chỉ tạo cơ hội cho nó đổi tên nhân vật.
pub fn chi_them_mot_dau_phu(gia: u32) -> bool {
    gia == GIA_THEM_DAU_PHU
}

/// Giá của phép đổi dấu phụ, tính **theo chiều** ở từng vị trí.
///
/// Hai chuỗi đưa vào luôn cùng độ dài — [`doi_dau_phu`] chỉ thay tại chỗ.
fn gia_doi_dau_phu(goc: &str, moi: &str) -> u32 {
    goc.chars()
        .zip(moi.chars())
        .map(|(a, b)| {
            if a == b {
                0
            } else if nen_cua(a).is_some() {
                GIA_DUNG_DAU_PHU
            } else {
                GIA_THEM_DAU_PHU
            }
        })
        .sum()
}

/// Trần số ứng viên trả về.
///
/// Nới rộng khi thêm phép chèn và phép thay chữ: danh sách thô giờ lớn hơn
/// nhiều, nhưng tầng trên còn lọc tiếp bằng từ điển rồi mới dùng, nên cắt sớm ở
/// đây là cắt mất đúng ứng viên mà từ điển sẽ xác nhận.
///
/// **Đừng nới thêm nữa.** Nghe thì càng rộng càng an toàn — cắt ở đây là cắt theo
/// **giá**, mà bằng chứng mạnh nhất (từ ghép trong từ điển) thì tầng trên mới hỏi,
/// nên ứng viên đắt mà đúng có thể bị loại trước khi ai kịp xác nhận nó. Đã thử
/// 80: số lỗi bắt được **tăng** (127 so với 126) mà chất lượng **tụt**, vì phần
/// lớn ứng viên mới cũng có trong từ điển nên chúng lọt tới tay mô hình và cho nó
/// thêm đường chọn sai. Đo trên tập 4 Harry Potter, 5 quyết định đổi và 4 đổi theo
/// hướng xấu:
///
/// | | 40 | 80 |
/// |---|---|---|
/// | `ẩnh` | `ảnh` | `ăn` |
/// | `kó` (3 chỗ) | `có` cả ba | `tớ`, `có`, `tớ` |
/// | `Bàc` | `Bạn` | `Báo` |
///
/// Số lượt chấm cũng đi từ 1.427 lên 1.970 (+38%). Nói cách khác: chỗ nghẽn không
/// phải bộ sinh ứng viên, mà là tầng chọn — cho nó nhiều lựa chọn hơn không làm nó
/// chọn khá hơn.
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

    // Bộ 0 — bỏ ký tự lặp. Rẻ nhất bảng, xem [`GIA_BO_LAP`].
    let khong_thanh: String = thap.chars().map(|c| am_tiet::bo_thanh(c).0).collect();
    for bt in bo_ky_tu_lap(&khong_thanh) {
        them_moi_thanh(&bt, thanh_goc, GIA_BO_LAP, &mut ra);
    }

    // Bộ 1 — đổi dấu phụ, giữ nguyên bộ khung chữ cái.
    for bt in doi_dau_phu(&khong_thanh) {
        them_moi_thanh(&bt, thanh_goc, gia_doi_dau_phu(&khong_thanh, &bt), &mut ra);
    }

    // Bộ 2 — thêm/bớt/đảo/thay một chữ cái. Đắt hơn bộ 1 vì đây là gõ trượt
    // phím, hiếm hơn là quên dấu.
    for (bt, gia_sua) in sua_mot_chu(&khong_thanh) {
        them_moi_thanh(&bt, thanh_goc, gia_sua, &mut ra);
        // Đổi dấu phụ *sau khi* thêm bớt chữ: `nguoiw` kiểu gõ hỏng cần cả hai.
        for bt2 in doi_dau_phu(&bt) {
            let g = gia_sua + gia_doi_dau_phu(&bt, &bt2);
            them_moi_thanh(&bt2, thanh_goc, g, &mut ra);
        }
    }

    // Phụ thu cho ứng viên **đổi số chữ** so với bản gốc.
    //
    // Cùng một giá thì cách nào giữ nguyên số chữ người ta gõ là cách gần bản
    // gốc hơn. Không có phụ thu này thì `đing` cho `đin` (xoá `g`) và `đinh`
    // (đổi `g` thành `h`) hoà nhau, rồi `đin` thắng vì ngắn hơn nên đứng trước
    // trong bảng chữ cái.
    let so_chu_goc = thap.chars().count();
    for u in ra.iter_mut() {
        if u.chu.chars().count() != so_chu_goc {
            u.gia += 1;
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
    fn bo_ky_tu_lap_re_hon_moi_cach_sua_khac() {
        // Tiếng Việt gần như không có hai ký tự giống hệt nhau đứng cạnh nhau, nên
        // gặp `gg`/`ii`/`nn`/`tt` thì đó là phím bấm hai lần — không cần cân nhắc
        // cách sửa nào khác trước nó.
        for (sai, dung) in
            [("khôngg", "không"), ("phảii", "phải"), ("mộtt", "một"), ("nhưnng", "nhưng")]
        {
            assert_eq!(dau_bang(sai), dung, "{:?}", sinh(sai));
            assert_eq!(gia_cua(sai, dung), Some(1), "{:?}", sinh(sai));
        }
    }

    #[test]
    fn bo_ca_dau_thanh_thi_dat_hon_chi_doi_dau_phu() {
        // `ẩnh` → `ảnh` đổi `â` thành `a` và **giữ** dấu hỏi. `ẩnh` → `anh` đổi
        // `â` thành `a` **và** bỏ luôn dấu hỏi. Hai việc phải đắt hơn một việc.
        assert_eq!(gia_cua("ẩnh", "ảnh"), Some(3), "{:?}", sinh("ẩnh"));
        assert_eq!(gia_cua("ẩnh", "anh"), Some(6), "{:?}", sinh("ẩnh"));
    }

    #[test]
    fn dau_thanh_khong_lam_hai_ky_tu_thanh_khac_nhau() {
        // `ó` là `o` mang dấu thanh, không phải một chữ cái khác `o`. Nên `xoóng`
        // cũng là `oo` và cũng được ngoại lệ che — phép bỏ ký tự lặp chạy trên bộ
        // khung đã bỏ dấu thanh nên chuyện này đúng sẵn, bài kiểm chỉ ghim nó lại.
        for x in ["xoong", "xoóng", "boong"] {
            let bo_bot = sinh(x).into_iter().filter(|u| u.chu.chars().count() < 5);
            assert!(
                !bo_bot.clone().any(|u| u.gia <= 1),
                "đã bỏ một chữ `o` của `{x}`: {:?}",
                bo_bot.collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn u_va_u_moc_la_hai_chu_khac_nhau_nen_uu_khong_phai_lap() {
        // `ư` và `u` khác nhau, nên `ưu` không phải cặp lặp.
        assert!(bo_ky_tu_lap("hưu").is_empty());
        // Còn `ưư` **là** cặp lặp, nhưng phép bỏ lặp vẫn không nhận: chữ mang dấu
        // phụ thì để tầng dấu phụ lo. `lưư` phải ra `lưu` (dấu móc rơi thêm sang
        // chữ bên cạnh), không phải `lư` (bấm hai lần một phím).
        assert!(bo_ky_tu_lap("lưư").is_empty());
        assert!(co("lưư", "lưu"), "{:?}", sinh("lưư"));
        assert!(gia_cua("lưư", "lưu") < gia_cua("lưư", "lư"), "{:?}", sinh("lưư"));
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

    /// Giá của một ứng viên cụ thể, `None` nếu không sinh ra.
    fn gia_cua(tieng: &str, uv: &str) -> Option<u32> {
        sinh(tieng).iter().find(|u| u.chu == uv).map(|u| u.gia)
    }

    #[test]
    fn them_dau_phu_re_hon_dung_vao_dau_phu_da_co() {
        // Đây là toàn bộ phần bất đối xứng có căn cứ. Quên phím phụ là lỗi thường
        // gặp nhất của tiếng Việt gõ máy, nên `may` → `mây` phải rẻ. Chiều ngược
        // lại đòi người ta bấm **thêm** một phím không cần, nên `mây` → `may` phải
        // đắt hơn.
        assert!(
            gia_cua("may", "mây") < gia_cua("mây", "may"),
            "{:?} / {:?}",
            gia_cua("may", "mây"),
            gia_cua("mây", "may")
        );

        // Và giữ nguyên chữ đã mang dấu phụ phải rẻ hơn đụng vào nó: `Duơng` chỉ
        // cần thêm dấu vào `u` và để `ơ` yên. Đây là ca thật trên sách — bản chấm
        // cả câu từng đổi nó thành `Hương`, tức là đổi tên nhân vật.
        assert!(gia_cua("duơng", "dương") < gia_cua("duơng", "duông"));
    }

    #[test]
    fn a_trang_va_a_mu_la_hai_chu_khac_nhau_nen_khong_ai_hon_ai() {
        // `ă` và `â` không phải hai biến thể của nhau, chúng là hai chữ khác nhau.
        // Đổi `ă` thành `â` cũng là đổi sang một chữ khác y như đổi thành `a`, nên
        // hai cách phải **cùng giá**.
        //
        // Xếp `ă` → `â` rẻ hơn `ă` → `a` thì nghe như "ưu tiên giữ dấu phụ", nhưng
        // đó là bịa ra một sự thiên vị không có căn cứ: lập luận Telex chỉ nói quên
        // phím phụ thì thường, nó không nói gõ `aw` nhầm thành `aa` thì hay hơn gõ
        // thừa `w`. Chọn giữa `may` và `mây` là việc của tầng từ ghép — trong `Trò
        // măy mắn lắm đó` thì `may mắn` có trong từ điển, `mây mắn` thì không.
        assert_eq!(gia_cua("măy", "may"), gia_cua("măy", "mây"), "{:?}", sinh("măy"));
        assert!(gia_cua("măy", "may").is_some(), "mất hẳn ứng viên: {:?}", sinh("măy"));
    }

    #[test]
    fn khong_cat_mat_ung_vien_ma_tu_ghep_se_xac_nhan() {
        // `màn cẳu sổ` phải ra `màn cửa sổ`, và bằng chứng ấy nằm ở tầng từ ghép
        // chứ không ở giá: `cửa sổ` có trong từ điển, `của sổ` thì không.
        //
        // Nhưng tầng từ ghép chỉ chọn được trong danh sách mà [`sinh`] trả về, mà
        // [`sinh`] cắt danh sách **theo giá** ở [`MAX_UNG_VIEN`]. `cửa` cần cả một
        // phép thêm dấu phụ lẫn một phép mất dấu phụ nên nó đắt, và khi
        // [`GIA_DUNG_DAU_PHU`] tăng lên thì nó bị đẩy khỏi cửa sổ — cả cuốn sách
        // ra `của sổ` mà không tầng nào báo gì. Đây là bài kiểm giữ cửa cho chỗ ấy.
        assert!(co("cẳu", "cửa"), "{:?}", sinh("cẳu"));
        assert!(co("cẳu", "của"), "{:?}", sinh("cẳu"));
    }

    #[test]
    fn van_bo_duoc_dau_phu_khi_dau_roi_vao_chu_ben_canh() {
        // Luật trên là luật **ưu tiên**, không phải luật cấm — khác `đ`. Lớp lỗi
        // "dấu móc rơi vào chữ bên cạnh" chỉ sửa được bằng cách bỏ dấu phụ:
        // `thưộc` → `thuộc`, `cưồng` → `cuồng`, `xưống` → `xuống`. Đo trên hai bộ
        // truyện thì quá nửa số phép bỏ dấu phụ là loại này, nên chặn hẳn là mất
        // cả lớp ấy.
        for (sai, dung) in [("thưộc", "thuộc"), ("cưồng", "cuồng"), ("xưống", "xuống")] {
            assert!(co(sai, dung), "mất `{dung}`: {:?}", sinh(sai));
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
