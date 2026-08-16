//! Cấu tạo âm tiết tiếng Việt: tách một tiếng thành *âm đầu + vần + thanh*, rồi
//! ghép ngược lại.
//!
//! Vì sao phải mô hình hoá thay vì tra từ điển: từ điển âm tiết đầy đủ có khoảng
//! 6.800 mục, mà cái ta cần bắt lại là những tiếng **không tồn tại** — và tiếng
//! không tồn tại thì nhiều vô kể, không liệt kê được. Cấu tạo thì hữu hạn: 27 âm
//! đầu × 156 vần × 6 thanh, kèm vài luật chính tả. Tiếng nào ghép được từ ba
//! thành phần ấy là hợp lệ, không thì sai — và ta biết **sai ở đâu**, nên sinh
//! được ứng viên sửa thay vì chỉ báo đỏ.
//!
//! Cách này bắt đúng lớp lỗi hay gặp nhất trong ebook: `thuơng` (vần `uơ` không
//! đi với `ng`), `khôngg`, `nguơi`, `qùa`, `giừ`. Nó **không** bắt được tiếng
//! đúng cấu tạo mà không phải từ (`ngìn`) — phần ấy để tầng mô hình ngôn ngữ lo.

/// Thanh điệu. Số hiệu trùng thứ tự quen thuộc nên đọc log dễ hơn dùng enum tên dài.
pub const NGANG: u8 = 0;
pub const HUYEN: u8 = 1;
pub const SAC: u8 = 2;
pub const HOI: u8 = 3;
pub const NGA: u8 = 4;
pub const NANG: u8 = 5;

/// Bảng nguyên âm: mỗi dòng là một nguyên âm gốc kèm sáu dạng mang thanh.
/// Thứ tự trong dòng đúng bằng số hiệu thanh ở trên.
const BANG_NGUYEN_AM: [[char; 6]; 12] = [
    ['a', 'à', 'á', 'ả', 'ã', 'ạ'],
    ['ă', 'ằ', 'ắ', 'ẳ', 'ẵ', 'ặ'],
    ['â', 'ầ', 'ấ', 'ẩ', 'ẫ', 'ậ'],
    ['e', 'è', 'é', 'ẻ', 'ẽ', 'ẹ'],
    ['ê', 'ề', 'ế', 'ể', 'ễ', 'ệ'],
    ['i', 'ì', 'í', 'ỉ', 'ĩ', 'ị'],
    ['o', 'ò', 'ó', 'ỏ', 'õ', 'ọ'],
    ['ô', 'ồ', 'ố', 'ổ', 'ỗ', 'ộ'],
    ['ơ', 'ờ', 'ớ', 'ở', 'ỡ', 'ợ'],
    ['u', 'ù', 'ú', 'ủ', 'ũ', 'ụ'],
    ['ư', 'ừ', 'ứ', 'ử', 'ữ', 'ự'],
    ['y', 'ỳ', 'ý', 'ỷ', 'ỹ', 'ỵ'],
];

/// Nguyên âm có dấu phụ (mũ, móc, trăng). Dùng cho luật đặt dấu thanh: khi vần
/// có nguyên âm mang dấu phụ thì dấu thanh gần như luôn rơi vào đó.
const CO_DAU_PHU: [char; 6] = ['ă', 'â', 'ê', 'ô', 'ơ', 'ư'];

/// Âm đầu, xếp **dài trước ngắn** vì phép tách dùng khớp tham lam: để `n` đứng
/// trước `ng` thì `ngà` bị tách thành `n` + `gà`.
const AM_DAU: [&str; 27] = [
    "ngh", "ng", "nh", "ch", "tr", "th", "ph", "kh", "gh", "gi", "qu", "b", "c", "d", "đ", "g",
    "h", "k", "l", "m", "n", "p", "r", "s", "t", "v", "x",
];

/// Vần (không mang thanh), gom theo âm cuối.
///
/// Danh sách này là **đóng**: tiếng Việt không sinh thêm vần mới. Vài vần chỉ
/// gặp trong từ mượn hoặc từ địa phương (`oong` trong "xoong", `ooc` trong
/// "coóc-xê", `uơ` trong "huơ") vẫn để vào, vì thiếu chúng thì một từ đúng bị
/// báo sai — mà báo sai ở đây nghĩa là **tự động sửa hỏng** một chỗ vốn đúng.
const VAN: &[&str] = &[
    // Không có âm cuối
    "a", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y", "ia", "ua", "ưa", "uơ", "oa", "oe", "uê",
    "uy", "uya",
    // Âm cuối bán nguyên âm -i/-y
    "ai", "ay", "ây", "oi", "ôi", "ơi", "ui", "ưi", "oai", "oay", "uây", "uôi", "ươi",
    // Âm cuối bán nguyên âm -u/-o
    "ao", "au", "âu", "eo", "êu", "iu", "ưu", "iêu", "yêu", "ươu", "oao", "oeo", "uyu",
    // Âm cuối -m
    "am", "ăm", "âm", "em", "êm", "im", "om", "ôm", "ơm", "um", "ưm", "ươm", "iêm", "yêm", "uôm",
    "oam", "oăm", "oem",
    // Âm cuối -n
    "an", "ăn", "ân", "en", "ên", "in", "on", "ôn", "ơn", "un", "ưn", "iên", "yên", "uôn", "ươn",
    "oan", "oăn", "oen", "uân", "uyên",
    // Âm cuối -ng
    "ang", "ăng", "âng", "eng", "êng", "ong", "ông", "ung", "ưng", "iêng", "uông", "ương", "oang",
    "oăng", "oong", "uâng",
    // Âm cuối -nh
    "anh", "ênh", "inh", "oanh", "uynh", "uênh",
    // Âm cuối -p
    "ap", "ăp", "âp", "ep", "êp", "ip", "op", "ôp", "ơp", "up", "ươp", "iêp", "oap",
    // Âm cuối -t
    "at", "ăt", "ât", "et", "êt", "it", "ot", "ôt", "ơt", "ut", "ưt", "iêt", "yêt", "uôt", "ươt",
    "oat", "oăt", "oet", "uât", "uyt", "uyêt",
    // Âm cuối -c
    //
    // `ec`, `êc`, `ic` trông lạ mắt nên bản đầu quên mất, mà chúng có thật:
    // `méc`, `xéc`, `nhếc`, `híc`, `nhích` (dạng `hic`). Thiếu một vần là **mọi
    // tiếng mang vần ấy đều bị báo sai rồi bị sửa thành tiếng khác** — lấy chữ
    // đúng của tác giả đổi thành chữ sai, hỏng ngược hẳn với việc app này làm.
    // Tìm ra bằng `examples/soi_van.rs`, quét chín triệu tiếng trong sách thật.
    "ac", "ăc", "âc", "ec", "êc", "ic", "oc", "ôc", "uc", "ưc", "iêc", "uôc", "ươc", "oac", "oăc",
    "oec", "ooc",
    // Âm cuối -ch
    "ach", "êch", "ich", "oach", "uêch", "uych",
];

/// Vần khép (âm cuối là phụ âm tắc) chỉ mang được thanh sắc hoặc nặng — đây là
/// luật ngữ âm, không phải quy ước chính tả: âm tắc cắt ngang luồng hơi nên
/// không giữ nổi đường nét của huyền, hỏi, ngã. `bàt`, `bảc`, `bãch` sai chắc.
fn van_khep(van: &str) -> bool {
    van.ends_with("ch") || van.ends_with('p') || van.ends_with('t') || van.ends_with('c')
}

/// Một âm tiết đã tách xong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmTiet {
    pub am_dau: String,
    /// Vần **không mang thanh**, viết thường.
    pub van: String,
    pub thanh: u8,
    /// Chữ đầu có viết hoa không, và cả tiếng có viết hoa không — giữ lại để
    /// ghép ngược đúng dạng người ta đã viết.
    pub hoa_dau: bool,
    pub hoa_het: bool,
}

/// Bỏ thanh khỏi một nguyên âm, trả về `(nguyên âm gốc, thanh)`.
/// Ký tự không phải nguyên âm tiếng Việt thì trả nguyên nó với thanh ngang.
pub fn bo_thanh(c: char) -> (char, u8) {
    for dong in BANG_NGUYEN_AM.iter() {
        for (thanh, &co) in dong.iter().enumerate() {
            if co == c {
                return (dong[0], thanh as u8);
            }
        }
    }
    (c, NGANG)
}

/// Gắn thanh vào một nguyên âm gốc. Không phải nguyên âm thì trả nguyên nó.
pub fn gan_thanh(c: char, thanh: u8) -> char {
    for dong in BANG_NGUYEN_AM.iter() {
        if dong[0] == c {
            return dong[thanh as usize];
        }
    }
    c
}

/// Ký tự có phải nguyên âm tiếng Việt (kể cả dạng mang thanh) không.
pub fn la_nguyen_am(c: char) -> bool {
    let (goc, _) = bo_thanh(c);
    BANG_NGUYEN_AM.iter().any(|d| d[0] == goc)
}

/// Ký tự có thuộc bảng chữ cái tiếng Việt không (chữ cái Latin + nguyên âm có dấu + đ).
pub fn la_chu_viet(c: char) -> bool {
    c.is_ascii_alphabetic() || la_nguyen_am(c) || c == 'đ' || c == 'Đ'
}

/// Bỏ toàn bộ dấu thanh khỏi một chuỗi, giữ nguyên dấu phụ (â vẫn là â).
pub fn bo_thanh_chuoi(s: &str) -> String {
    s.chars().map(|c| bo_thanh(c).0).collect()
}

/// Bỏ cả dấu thanh lẫn dấu phụ, đưa về ASCII. Dùng để so hai từ "cùng gốc chữ".
pub fn bo_het_dau(s: &str) -> String {
    s.chars()
        .map(|c| {
            let (goc, _) = bo_thanh(c);
            match goc {
                'ă' | 'â' => 'a',
                'ê' => 'e',
                'ô' | 'ơ' => 'o',
                'ư' => 'u',
                'đ' => 'd',
                'Đ' => 'D',
                k => k,
            }
        })
        .collect()
}

/// Chẻ một **khung chữ không mang thanh** thành âm đầu + vần.
///
/// Tách khỏi [`tach`] vì hai bên hỏi hai câu khác nhau. `tach` hỏi "chữ này
/// viết đúng không", nên nó loại `hoat` — vần khép mà không mang dấu thanh thì
/// không phải tiếng Việt. Còn phần sinh ứng viên sửa hỏi "khung này chẻ ra thế
/// nào", và nó **cần** chẻ được `hoat` để rồi gắn dấu vào thành `hoạt`.
///
/// Gộp hai câu hỏi ấy làm một là lỗi đã xảy ra một lần: phần sinh ứng viên gọi
/// `tach` trên khung không dấu, nên mọi tiếng có vần khép — `hoạt`, `biết`,
/// `các`, `một` — không bao giờ sinh nổi một cách sửa nào, mà im lặng.
pub fn tach_khung(khung: &str) -> Option<(String, String)> {
    if khung.is_empty() || !khung.chars().all(la_chu_viet) {
        return None;
    }
    let mut am_dau = "";
    for &ad in AM_DAU.iter() {
        if khung.starts_with(ad) {
            am_dau = ad;
            break;
        }
    }
    // `van_viet` là phần **như đã viết ra**; `van` là vần thật sau khi trả lại
    // chữ mà âm đầu nuốt mất. Hai thứ này khác nhau đúng ở ca `quýt`/`giếng`,
    // và luật chính tả bên dưới phải xét bản viết: luật cấm `qu` đi với vần bắt
    // đầu bằng `u` là cấm viết `quuýt`, chứ không cấm chính chữ `quýt`.
    let van_viet = &khung[am_dau.len()..];
    let mut van = van_viet;
    let bu;
    if let Some(c) = chu_bi_nuot(am_dau) {
        if !VAN.contains(&van) {
            bu = format!("{c}{van}");
            if VAN.contains(&bu.as_str()) {
                van = &bu;
            }
        }
    }
    if !VAN.contains(&van) || !hop_le_am_dau_van(am_dau, van_viet) {
        return None;
    }
    Some((am_dau.to_string(), van.to_string()))
}

/// Chữ mà âm đầu này **nuốt mất** của vần đi sau, nếu có.
///
/// Hai âm đầu viết bằng hai chữ mà chữ sau lại trùng với chữ mở đầu vần, nên
/// chính tả viết gộp làm một:
///
/// | Âm đầu | Vần | Viết ra | Chứ không phải |
/// |---|---|---|---|
/// | `gi` | `iêng` | `giếng` | `giiếng` |
/// | `gi` | `in` | `gìn` | `giìn` |
/// | `qu` | `uyt` | `quýt` | `quuýt` |
/// | `qu` | `uynh` | `quỳnh` | `quuỳnh` |
///
/// Nên khi phần còn lại sau âm đầu không thành vần, phải thử trả lại chữ đã bị
/// nuốt. Bản đầu chỉ xử lý `gi` mà quên `qu`, nên `quýt` và `quỳnh` bị báo sai —
/// 85 lần trong ba cuốn sách đo được.
fn chu_bi_nuot(am_dau: &str) -> Option<char> {
    match am_dau {
        "gi" => Some('i'),
        "qu" => Some('u'),
        _ => None,
    }
}

/// Thanh này gắn vào vần kia có hợp lệ không.
///
/// Vần khép (âm cuối `p`, `t`, `c`, `ch`) chỉ mang được sắc hoặc nặng: âm tắc
/// cắt ngang luồng hơi nên không giữ nổi đường nét của huyền, hỏi, ngã.
pub fn thanh_hop_le(van: &str, thanh: u8) -> bool {
    if van_khep(van) {
        thanh == SAC || thanh == NANG
    } else {
        true
    }
}

/// Tách một tiếng thành âm đầu + vần + thanh.
///
/// Trả `None` khi tiếng không ghép được từ ba thành phần hợp lệ — tức là sai
/// chính tả về mặt cấu tạo, hoặc không phải tiếng Việt.
pub fn tach(tieng: &str) -> Option<AmTiet> {
    if tieng.is_empty() {
        return None;
    }
    let hoa_dau = tieng.chars().next().is_some_and(|c| c.is_uppercase());
    let hoa_het = tieng.chars().filter(|c| c.is_alphabetic()).count() > 1
        && tieng.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase());

    let thap = tieng.to_lowercase();
    if !thap.chars().all(la_chu_viet) {
        return None;
    }

    // Gỡ thanh ra trước: `bàn` và `ban` cùng một vần, chỉ khác thanh. Nếu tiếng
    // mang hai dấu thanh (`bàán`) thì đây là lỗi gõ, coi như không tách được.
    let mut thanh = NGANG;
    let mut so_dau = 0;
    let mut goc = String::with_capacity(thap.len());
    for c in thap.chars() {
        let (g, t) = bo_thanh(c);
        if t != NGANG {
            thanh = t;
            so_dau += 1;
        }
        goc.push(g);
    }
    if so_dau > 1 {
        return None;
    }

    // Khớp tham lam âm đầu. `AM_DAU` đã xếp dài trước ngắn.
    let mut am_dau = "";
    for &ad in AM_DAU.iter() {
        if goc.starts_with(ad) {
            am_dau = ad;
            break;
        }
    }
    let van_viet = &goc[am_dau.len()..];
    let mut van = van_viet;

    // Trả lại chữ mà âm đầu đã nuốt — xem [`chu_bi_nuot`].
    let bu;
    if let Some(c) = chu_bi_nuot(am_dau) {
        if !VAN.contains(&van) {
            bu = format!("{c}{van}");
            if VAN.contains(&bu.as_str()) {
                van = &bu;
            }
        }
    }

    if !VAN.contains(&van) {
        return None;
    }
    if van_khep(van) {
        // Vần khép (âm cuối tắc) chỉ mang được sắc hoặc nặng. Thanh ngang cũng
        // loại, nhưng vì lý do khác: `bat`, `internet` là chữ chưa bỏ dấu hoặc
        // từ mượn — chỗ này chỉ báo *không tách được*, còn có phải lỗi hay
        // không thì tầng trên quyết.
        if thanh != SAC && thanh != NANG {
            return None;
        }
    }
    if !hop_le_am_dau_van(am_dau, van_viet) {
        return None;
    }

    Some(AmTiet {
        am_dau: am_dau.to_string(),
        van: van.to_string(),
        thanh,
        hoa_dau,
        hoa_het,
    })
}

/// Luật chính tả ràng buộc âm đầu với vần đi sau.
///
/// Ba cặp âm đầu dưới đây phát âm giống hệt nhau, chỉ khác cách viết, và cách
/// viết nào thì do nguyên âm đứng sau quyết định — nên viết sai là lỗi chắc
/// chắn, sửa được không cần ngữ cảnh.
fn hop_le_am_dau_van(am_dau: &str, van: &str) -> bool {
    let dau_van = van.chars().next().unwrap_or(' ');
    // Đứng trước i, e, ê, y phải viết k / gh / ngh.
    let truoc_hep = matches!(dau_van, 'i' | 'e' | 'ê' | 'y');
    match am_dau {
        "k" | "gh" | "ngh" => truoc_hep,
        "c" | "g" | "ng" => !truoc_hep,
        // `qu` đã nuốt mất chữ u, nên vần đi sau không được bắt đầu bằng u/o nữa:
        // `quoan`, `quuân` không tồn tại.
        "qu" => !matches!(dau_van, 'u' | 'o'),
        _ => true,
    }
}

/// Ghép âm tiết trở lại thành chữ, đặt dấu thanh đúng chỗ.
pub fn ghep(at: &AmTiet, kieu_moi: bool) -> String {
    let mut s = String::with_capacity(am_do_dai(at));
    // Chỗ viết gộp: âm đầu nuốt chữ đầu của vần — xem [`chu_bi_nuot`].
    // `gi`+`iêng`→`giếng`, `qu`+`uyt`→`quýt`.
    if chu_bi_nuot(&at.am_dau).is_some() && chu_bi_nuot(&at.am_dau) == at.van.chars().next() {
        s.push_str(&at.am_dau[..at.am_dau.len() - 1]);
    } else {
        s.push_str(&at.am_dau);
    }
    let vi_tri = vi_tri_dau_thanh(&at.am_dau, &at.van, kieu_moi);
    for (i, c) in at.van.chars().enumerate() {
        s.push(if i == vi_tri { gan_thanh(c, at.thanh) } else { c });
    }
    if at.hoa_het {
        s.to_uppercase()
    } else if at.hoa_dau {
        let mut ch = s.chars();
        match ch.next() {
            Some(d) => d.to_uppercase().collect::<String>() + ch.as_str(),
            None => s,
        }
    } else {
        s
    }
}

fn am_do_dai(at: &AmTiet) -> usize {
    at.am_dau.len() + at.van.len() + 2
}

/// Dấu thanh đặt vào nguyên âm thứ mấy của vần (đếm theo ký tự).
///
/// `kieu_moi` chỉ đổi kết quả ở đúng ba vần mở `oa`, `oe`, `uy`: kiểu cũ viết
/// `hòa`, `khòe`, `thùy`; kiểu mới viết `hoà`, `khoè`, `thuỳ`. Cả hai đều được
/// công nhận, nên đây là chuyện **thống nhất trong một cuốn sách** chứ không
/// phải chuyện đúng sai — chỗ khác dùng cái này để kéo cả sách về một kiểu.
pub fn vi_tri_dau_thanh(am_dau: &str, van: &str, kieu_moi: bool) -> usize {
    let ky_tu: Vec<char> = van.chars().collect();
    let nguyen_am: Vec<usize> = (0..ky_tu.len()).filter(|&i| la_nguyen_am(ky_tu[i])).collect();
    if nguyen_am.is_empty() {
        return 0;
    }
    if nguyen_am.len() == 1 {
        return nguyen_am[0];
    }

    // Nguyên âm mang dấu phụ hút dấu thanh về mình: `iê`→ê, `uô`→ô, `ưa`→ư.
    // Vần `ươ` có hai chữ mang dấu phụ nên lấy chữ sau: `được`, `người`, `rượu`.
    if let Some(&i) = nguyen_am.iter().rev().find(|&&i| CO_DAU_PHU.contains(&ky_tu[i])) {
        return i;
    }

    let co_am_cuoi = nguyen_am.last().map(|&i| i + 1 < ky_tu.len()).unwrap_or(false);
    if co_am_cuoi {
        // Có âm cuối thì dấu rơi vào nguyên âm cuối cùng: `hoàn`, `khoảnh`.
        return *nguyen_am.last().unwrap();
    }

    // Không cần biệt đãi `qu`/`gi` ở đây: chữ u/i của chúng đã bị [`tach`] xếp
    // vào âm đầu rồi, nên vần còn lại chỉ toàn nguyên âm chính và luật chung
    // cho ra đúng `quý`, `giá`, `giày`. (Từng có nhánh riêng cho hai âm đầu này
    // và nó làm hỏng `giày` thành `giáy`.)
    let _ = am_dau;
    if kieu_moi && matches!(van, "oa" | "oe" | "uy") {
        return nguyen_am[nguyen_am.len() - 1];
    }
    nguyen_am[nguyen_am.len() - 2]
}

/// Tiếng này viết đúng cấu tạo không.
pub fn hop_le(tieng: &str) -> bool {
    tach(tieng).is_some()
}

/// Toàn bộ vần, cho phần sinh ứng viên sửa.
pub fn tat_ca_van() -> &'static [&'static str] {
    &VAN
}

/// Toàn bộ âm đầu, cho phần sinh ứng viên sửa.
pub fn tat_ca_am_dau() -> &'static [&'static str] {
    &AM_DAU
}

#[cfg(test)]
mod kiem {
    use super::*;

    #[test]
    fn tach_roi_ghep_lai_khong_doi() {
        // Kiểm cả vòng: chữ → thành phần → chữ. Sai ở đâu cũng lộ ra ngay.
        for tieng in [
            "không", "người", "được", "quyển", "nghiêng", "thuyền", "gì", "giá", "quả", "quý",
            "khuỷu", "rượu", "xoong", "huơ", "chuyện", "ngoằn", "tuyệt", "khoẻ",
            // Nhóm `gi` viết gộp chữ i — mỗi chữ một kiểu gộp khác nhau.
            "gìn", "giếng", "giêng", "giày", "giấu", "giòi",
        ] {
            let at = tach(tieng).unwrap_or_else(|| panic!("không tách được: {tieng}"));
            let lai = ghep(&at, true);
            assert_eq!(
                bo_thanh_chuoi(&lai),
                bo_thanh_chuoi(tieng),
                "phần chữ đổi: {tieng} → {lai}"
            );
        }
    }

    #[test]
    fn nhan_nhung_van_de_bi_bo_quen() {
        // Bốn vần này trông lạ nên bản đầu quên mất, và mỗi vần bị quên là mọi
        // tiếng mang nó bị sửa thành tiếng khác. Tìm ra bằng cách quét chín
        // triệu tiếng trong sách thật — xem `examples/soi_van.rs`.
        for tieng in ["méc", "éc", "xéc", "nhếc", "híc", "nhích", "hừm", "ừm"] {
            assert!(hop_le(tieng), "vần của `{tieng}` không có trong bảng");
        }
    }

    #[test]
    fn am_dau_qu_nuot_chu_u_cua_van() {
        // `quýt` là qu + uyt, `quỳnh` là qu + uynh — chính tả viết gộp hai chữ
        // u làm một. Bản đầu chỉ xử lý ca tương tự của `gi` mà quên `qu`, nên
        // hai chữ này bị báo sai 85 lần trong ba cuốn sách.
        for tieng in ["quýt", "quỳnh", "quýnh"] {
            assert!(hop_le(tieng), "`{tieng}` phải hợp lệ");
            let at = tach(tieng).unwrap();
            assert_eq!(ghep(&at, false), tieng, "ghép lại không ra chính nó");
        }
        // Và không được nhận bừa: `qu` + vần bắt đầu bằng `o` vẫn phải bị loại.
        assert!(!hop_le("quoan"));
    }

    #[test]
    fn bat_duoc_tieng_sai_cau_tao() {
        // `thuơng`: vần `uơ` không đi với âm cuối `ng` — đây là lỗi gõ hay gặp
        // nhất, người ta gõ `uo` rồi bỏ dấu móc nhầm chữ.
        assert!(!hop_le("thuơng"), "thuơng phải bị bắt");
        assert!(hop_le("thương"));
        assert!(!hop_le("khôngg"));
        assert!(!hop_le("nguơi"));
        assert!(!hop_le("bàt"), "vần khép không mang thanh huyền");
        assert!(!hop_le("kách"), "k không đứng trước a");
        assert!(!hop_le("nghành"), "ngh không đứng trước a");
        assert!(hop_le("ngành"));
    }

    #[test]
    fn dat_dau_thanh_sau_qu_va_gi() {
        // `qu`/`gi` nuốt chữ u/i làm âm đầu, nên dấu phải rơi xuống nguyên âm
        // sau nó. Viết `qúy`, `gía` là lỗi chắc chắn.
        let at = tach("quy").unwrap();
        assert_eq!(ghep(&AmTiet { thanh: SAC, ..at }, false), "quý");
        let at = tach("gia").unwrap();
        assert_eq!(ghep(&AmTiet { thanh: SAC, ..at }, false), "giá");
    }

    #[test]
    fn kieu_dat_dau_cu_va_moi() {
        let at = tach("hoa").unwrap();
        assert_eq!(ghep(&AmTiet { thanh: HUYEN, ..at.clone() }, false), "hòa");
        assert_eq!(ghep(&AmTiet { thanh: HUYEN, ..at }, true), "hoà");
        // Vần có âm cuối thì hai kiểu giống nhau, đừng đổi bừa.
        let at = tach("hoan").unwrap();
        assert_eq!(ghep(&AmTiet { thanh: HUYEN, ..at.clone() }, false), "hoàn");
        assert_eq!(ghep(&AmTiet { thanh: HUYEN, ..at }, true), "hoàn");
    }

    #[test]
    fn giu_nguyen_kieu_viet_hoa() {
        assert_eq!(ghep(&tach("Người").unwrap(), true), "Người");
        assert_eq!(ghep(&tach("VIỆT").unwrap(), true), "VIỆT");
        assert_eq!(ghep(&tach("việt").unwrap(), true), "việt");
    }
}
