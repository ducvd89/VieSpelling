//! Điều phối các tầng dò lỗi trên một đoạn văn.
//!
//! **Thứ tự các tầng không đổi chỗ được.** Chuẩn hoá Unicode phải chạy trước
//! mọi thứ, vì một chữ `ế` gõ rời (e + mũ + sắc) trông giống hệt chữ đúng nhưng
//! trượt mọi phép so; chạy bộ kiểm âm tiết trước khi dựng lại NFC thì cả đoạn
//! bị báo sai. Dọn khoảng trắng phải chạy **sau** dấu câu, vì luật dấu câu chèn
//! thêm khoảng trắng và có thể chèn ra chỗ đã có sẵn một cái.
//!
//! Mỗi tầng nhận chuỗi và trả về **danh sách phép sửa trên chính chuỗi ấy**,
//! rồi bộ điều phối áp xong mới chạy tầng sau. Chạy hết mọi tầng trên cùng một
//! chuỗi rồi áp một lượt thì nhanh hơn, nhưng các phép sửa sẽ chồng lên nhau ở
//! những chỗ hai tầng cùng nhìn thấy vấn đề, và cái bị bỏ lại là cái ngẫu nhiên.

use crate::chuan_hoa::{self, CaiDat};
use crate::dau_thanh::{self, Kieu};
use crate::de_nham;
use crate::sua::{ap_dung, DoChac, Loai, SuaDoi};
use crate::tach_tu::{self, DangTu};
use crate::tu_dien;
use crate::{am_tiet, ung_vien};
use std::ops::Range;

/// Vì sao một chỗ ngờ **không cần hỏi mô hình**.
///
/// Mỗi loại ở đây là một **sự thật về tiếng Việt**, không phải một ước lượng: từ
/// điển nói `chúng ta` là một từ còn `chừ ta` thì không; bảng giá nói `Duơng` →
/// `Dương` chỉ thêm một dấu phụ mà không đụng vào chữ nào. Chỗ nào có sự thật thì
/// đừng đem ra hỏi, vì hỏi chỉ tạo cơ hội cho mô hình phủ quyết sai.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BangChung {
    /// Không có gì dứt khoát — phải hỏi mô hình, hoặc để ngỏ.
    #[default]
    Khong,
    /// Chính chữ ấy có trong **bảng typo hay gặp**, và bảng chỉ ghi đúng một đáp
    /// án. Mạnh nhất trong mọi hạng: đây là quan sát trực tiếp, không phải suy
    /// luận — xem [`crate::typo`].
    Typo,
    /// Ứng viên đầu bảng dựng lại được một cụm trong **từ điển riêng của cuốn
    /// sách**. Xem [`gom_cum_ten_rieng`].
    CumTenRieng,
    /// Ứng viên đầu bảng ghép được với chữ bên cạnh thành một từ có trong từ điển.
    TuGhep,
    /// Ứng viên đầu bảng chỉ **thêm đúng một dấu phụ**, và là từ có thật.
    DauPhu,
    /// Hai tiếng dính liền, và cách chia dứt khoát.
    TachDinh,
}

impl BangChung {
    /// Mô tả ngắn cho nhật ký. `None` nghĩa là phải hỏi mô hình.
    pub fn mo_ta(self) -> Option<&'static str> {
        match self {
            BangChung::Khong => None,
            BangChung::Typo => Some("typo hay gặp"),
            BangChung::CumTenRieng => Some("cụm tên riêng của sách"),
            BangChung::TuGhep => Some("từ ghép trong từ điển"),
            BangChung::DauPhu => Some("chỉ thêm một dấu phụ"),
            BangChung::TachDinh => Some("hai tiếng dính liền"),
        }
    }
}

/// Một chỗ bộ dò không tự quyết được — phải hỏi mô hình ngôn ngữ.
#[derive(Debug, Clone)]
pub struct ChoXet {
    /// Vị trí trong chuỗi **kết quả** của [`KetQua::chu`].
    pub pham_vi: Range<usize>,
    pub goc: String,
    /// Các cách sửa, đã xếp hạng sơ bộ. Bản gốc **không** nằm trong đây.
    pub ung_vien: Vec<String>,
    pub loai: Loai,
    pub ly_do: String,
    /// Bằng chứng đủ mạnh để áp thẳng ứng viên đầu bảng, nếu có.
    ///
    /// Mạnh hơn mọi điểm số: `chúg ta` thì chỉ `chúng` ghép được thành `chúng ta`,
    /// và không cần hỏi thêm ai. Khác [`BangChung::Khong`] thì phép sửa tự áp
    /// được ngay cả khi không có mô hình ngôn ngữ.
    pub bang_chung: BangChung,
}

#[derive(Debug, Clone, Default)]
pub struct KetQua {
    pub chu: String,
    pub da_sua: Vec<SuaDoi>,
    pub cho_xet: Vec<ChoXet>,
}

/// Máy chấm điểm câu. Cài đặt thật nằm ở crate `mohinh`; để ở đây dạng trait
/// nên lõi chính tả không phụ thuộc vào llama.cpp và chạy test không cần mô hình.
pub trait ChamDiem {
    /// Điểm tự nhiên của câu — **càng cao càng tự nhiên**. Chỉ dùng để **so**
    /// các câu gần giống hệt nhau, nên thang đo tuyệt đối không quan trọng, miễn
    /// nó nhất quán giữa các lần gọi.
    fn cham(&self, cau: &str) -> f32;

    /// Điểm của chữ `dien` khi đặt vào **chỗ trống** giữa `truoc` và `sau`.
    ///
    /// Khác [`ChamDiem::cham`] ở chỗ `truoc` chỉ là ngữ cảnh để đọc, **không**
    /// tính vào điểm. Nhờ thế đưa thêm ngữ cảnh vào không làm loãng chênh lệch
    /// giữa các ứng viên, mà chênh lệch ấy là toàn bộ thứ ta cần.
    ///
    /// Mặc định quay về chấm cả chuỗi, để mô hình giả trong bài kiểm không phải
    /// biết gì về chuyện chỗ trống.
    fn cham_cho_trong(&self, truoc: &str, dien: &str, sau: &str) -> f32 {
        self.cham(&format!("{truoc}{dien}{sau}"))
    }
}

/// Cách hỏi mô hình.
///
/// Đo trên tập 4 Harry Potter, mỗi lối ở ngưỡng đo riêng cho nó (0,03 và 0,018),
/// đếm theo **loại sai** trên 8 chỗ hai lối quyết khác nhau:
///
/// | | sửa nhầm | bỏ sót | lỗi bắt được | thời gian |
/// |---|---|---|---|---|
/// | [`KieuCham::CaCau`] | 5 | 1 | 126 | 69 giây |
/// | [`KieuCham::ChoTrong`] | 3 | 0 | 125 | 114 giây |
///
/// Đếm tổng thì hai lối trông như nhau, nên phải đếm theo loại: **sửa nhầm đắt
/// hơn bỏ sót nhiều lần**, và đó là chỗ hai lối khác nhau thật.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KieuCham {
    /// Thay ứng viên vào rồi chấm **cả câu chứa nó**, so với câu gốc.
    ///
    /// **Mặc định**: bắt được nhiều hơn một chút và nhanh gần gấp đôi — 1,7 lần
    /// trên một cuốn, 2,6 lần trên bộ truyện dài đoạn.
    ///
    /// Chỗ yếu của nó là điểm số **không dùng làm độ tin cậy được**: mấy phép sửa
    /// nhầm lại đứng đầu bảng điểm (`zợi` → `sợi` +0,504, `ghứ` → `chữ` +0,715 —
    /// đều là giọng nhân vật người dịch cố ý viết chệch) trong khi phép sửa thật
    /// chỉ quanh +0,10. Thứ tự sai hướng thì nâng ngưỡng là mất phép sửa thật
    /// trước khi chặn được phép sửa nhầm: trên 12 ca bẫy, lối này đạt 7/12 chỉ
    /// trong khoảng ngưỡng 0,01…0,06 rồi tụt còn 3/12 ở ngưỡng 0,10.
    #[default]
    CaCau,
    /// Khoét chữ sai thành **chỗ trống**, đưa mô hình hai câu trước và hai câu
    /// sau, rồi chấm riêng phần điền vào cùng phần đuôi đi kèm. Bật bằng
    /// `vsc --cho-trong`, và ngưỡng đi kèm nó là 0,018.
    ///
    /// Điểm của nó xếp **đúng chiều** — phép sửa thật cao hơn phép sửa nhầm — nên
    /// nó giữ 6…7/12 ca bẫy suốt khoảng ngưỡng 0,01 tới 0,25, rộng gấp hai mươi
    /// lần lối kia, và trên bộ truyện dài nó phá ít tên riêng hơn: 25 phép đổi phụ
    /// âm đầu so với 30, trong đó chữ hoa 2 so với 4.
    ///
    /// Chưa lấy làm mặc định vì đỉnh của hai lối bằng nhau mà lối này chậm hơn
    /// nhiều. Đáng lấy khi nào ngưỡng phải chạy theo mô hình khác hoặc loại sách
    /// khác — lúc ấy một điểm số xếp đúng chiều mới đáng cái giá thời gian.
    ChoTrong,
}

/// Hai đoạn văn kề bên đoạn đang soát, dùng làm ngữ cảnh cho [`KieuCham::ChoTrong`].
///
/// Phải lấy từ ngoài vào vì [`BoSoat::soat`] làm việc trên **một đoạn**, mà tiểu
/// thuyết thì đầy đoạn chỉ có một câu thoại — "hai câu trước" mà bó trong đoạn
/// thì phần lớn trường hợp chẳng có câu nào.
#[derive(Debug, Clone, Copy, Default)]
pub struct NguCanh<'a> {
    pub truoc: &'a str,
    pub sau: &'a str,
}

#[derive(Debug, Clone)]
pub struct TuyChon {
    pub chuan_hoa: CaiDat,
    /// Kéo cả sách về một kiểu đặt dấu thanh.
    pub nhat_quan_dau_thanh: bool,
    /// Sửa tiếng sai cấu tạo.
    pub am_tiet_sai: bool,
    /// Dò cặp dễ nhầm.
    pub de_nham: bool,
    /// Đụng cả vào chữ không dấu. **Mặc định tắt** — không phân được `khong`
    /// (tiếng Việt thiếu dấu) với `window` (tiếng Anh) nếu chỉ nhìn một tiếng,
    /// mà sách dịch thì đầy tên riêng nước ngoài.
    pub chu_khong_dau: bool,
    /// Mô hình phải chấm hơn bao nhiêu thì mới được đổi, tính bằng
    /// **nats/ký tự**.
    ///
    /// Đây là **cái van an toàn của cả ứng dụng**, và nó chặn hai chỗ: cách sửa
    /// phải hơn bản gốc quá mức này, **và** nếu mô hình muốn lật ngược thứ tự
    /// mà tầng luật đã xếp thì cũng phải hơn quá mức này.
    ///
    /// Đơn vị là mỗi **ký tự** chứ không phải mỗi token — xem
    /// `mohinh::MoHinh::cham_that` về lý do. Nên con số nhỏ hơn hẳn mức quen
    /// thuộc: một token tiếng Việt cỡ bốn năm ký tự.
    ///
    /// **Con số này gắn với [`TuyChon::kieu_cham`], đổi lối chấm thì phải đổi
    /// theo.** Hai lối chia điểm cho hai khoảng dài khác nhau nên thang đo khác
    /// nhau: biên độ hơn-kém trung vị của lối điền chỗ trống là 0,059 so với 0,076
    /// của lối cả câu trên tập 4 Harry Potter (0,063 so với 0,086 trên Phàm Nhân
    /// Tu Tiên), tức là cùng một con số ngưỡng thì lối chỗ trống bị chặn chặt hơn
    /// chừng 1,3 lần. Mặc định 0,03 là con số của lối cả câu; lối điền chỗ trống
    /// cần 0,018, và `vsc --cho-trong` tự kéo theo con số ấy.
    ///
    /// **0,018 là khe giữa hai cuốn sách**, tìm bằng cách hạ dần ngưỡng rồi đọc
    /// từng phép sửa được thêm vào:
    ///
    /// - Phép sửa **nhầm** có biên độ cao nhất là `zăc` → `ắc` ở +0,014 (phiên âm
    ///   `zic-zăc`), rồi `zợi` → `sợi` ở +0,010 (giọng nhân vật người dịch cố ý
    ///   viết chệch), rồi `dành cho` → `giành cho` ở +0,008.
    /// - Phép sửa **đúng** có biên độ thấp nhất mà ta cần là `đựoc` → `được` ở
    ///   +0,021, và hạ tới 0,018 còn lấy về thêm 5 phép nữa trên Phàm Nhân Tu Tiên
    ///   (`tiéng` → `tiếng`, `Môt` → `Một`, `crắc` → `rắc`, `dành được` → `giành
    ///   được`, `dành lấy` → `giành lấy`) — đúng cả 5.
    ///
    /// Hạ thấp hơn nữa thì Phàm Nhân Tu Tiên còn được thêm 9 phép đúng, nhưng
    /// Harry Potter bắt đầu phá tên phiên âm và giọng nhân vật. Hai cuốn không hoà
    /// nhau được ở chỗ ấy, nên lấy đầu **chặt** của khoảng — sửa nhầm đắt hơn bỏ
    /// sót nhiều lần.
    pub nguong_mo_hinh: f32,
    /// Hỏi mô hình theo lối chấm cả câu hay lối điền chỗ trống.
    pub kieu_cham: KieuCham,
}

impl Default for TuyChon {
    fn default() -> Self {
        TuyChon {
            chuan_hoa: CaiDat::default(),
            nhat_quan_dau_thanh: true,
            am_tiet_sai: true,
            de_nham: true,
            chu_khong_dau: false,
            // Cặp đôi: ngưỡng này đo cho lối chấm ngay dưới nó. Đổi một cái mà
            // để cái kia thì van an toàn lệch thang đo — xem `nguong_mo_hinh`.
            nguong_mo_hinh: 0.03,
            kieu_cham: KieuCham::CaCau,
        }
    }
}

pub struct BoSoat {
    pub tuy_chon: TuyChon,
    bang_de_nham: de_nham::Bang,
    bang_typo: crate::typo::Bang,
    kieu: Kieu,
    ten_rieng: std::collections::HashSet<String>,
    cum_ten_rieng: std::collections::HashSet<String>,
    tu_dung: std::collections::HashSet<String>,
}

impl BoSoat {
    pub fn moi(tuy_chon: TuyChon, kieu: Kieu) -> BoSoat {
        BoSoat {
            tuy_chon,
            bang_de_nham: de_nham::Bang::nap(),
            bang_typo: crate::typo::Bang::nap(),
            kieu,
            ten_rieng: Default::default(),
            cum_ten_rieng: Default::default(),
            tu_dung: Default::default(),
        }
    }

    /// Danh sách tên riêng đếm được từ chính cuốn sách — xem [`gom_ten_rieng`].
    pub fn voi_ten_rieng(mut self, ten: std::collections::HashSet<String>) -> BoSoat {
        self.ten_rieng = ten;
        self
    }

    /// Từ điển riêng đếm được từ chính cuốn sách — xem [`gom_cum_sach`].
    pub fn voi_cum_ten_rieng(mut self, cum: std::collections::HashSet<String>) -> BoSoat {
        self.cum_ten_rieng = cum;
        self
    }

    /// Vốn từ đếm được từ chính cuốn sách — xem [`gom_tu_dung`].
    pub fn voi_tu_dung(mut self, tu: std::collections::HashSet<String>) -> BoSoat {
        self.tu_dung = tu;
        self
    }

    /// Chạy mọi tầng không cần ngữ cảnh trên một đoạn.
    pub fn soat(&self, van_ban: &str) -> KetQua {
        let tc = &self.tuy_chon;
        let mut kq = KetQua::default();
        let mut chu = van_ban.to_string();

        // Tầng 1 — Unicode. Phải đứng đầu.
        let mut s = chuan_hoa::soat_unicode(&chu, &tc.chuan_hoa);
        chu = ghi_nhan(&chu, &mut s, &mut kq);
        if tc.chuan_hoa.unicode {
            if let Some(nfc) = chuan_hoa::dung_lai_nfc(&chu) {
                // Ghi **từng chữ đổi**, không ghi cả đoạn làm một phép sửa.
                // NFC gộp ký tự nên đoạn nào có một chữ gõ rời là cả đoạn khác
                // đi; ghi thô thì báo cáo đầy những dòng dài hàng trăm chữ mà
                // hai vế nhìn giống hệt nhau — đúng nhưng đọc không ra gì.
                for k in crate::doi_chieu::so(&chu, &nfc) {
                    kq.da_sua.push(SuaDoi::moi(
                        k.cu.clone(),
                        chu[k.cu.clone()].to_string(),
                        k.moi,
                        Loai::Unicode,
                        DoChac::Chac,
                        "dựng lại tổ hợp Unicode (NFC)",
                    ));
                }
                chu = nfc;
            }
        }

        // Tầng 2 — dấu câu, rồi mới tới khoảng trắng.
        let mut s = chuan_hoa::soat_dau_cau(&chu, &tc.chuan_hoa);
        chu = ghi_nhan(&chu, &mut s, &mut kq);
        let mut s = chuan_hoa::soat_khoang_trang(&chu, &tc.chuan_hoa);
        chu = ghi_nhan(&chu, &mut s, &mut kq);

        // Tầng 3 — dấu thanh.
        let mut s = dau_thanh::soat(&chu, self.kieu, tc.nhat_quan_dau_thanh);
        chu = ghi_nhan(&chu, &mut s, &mut kq);

        // Tầng 4 — cặp dễ nhầm. Phần chắc thì áp, phần tuỳ nghĩa để dành.
        if tc.de_nham {
            let tat_ca = self.bang_de_nham.soat(&chu);
            let (mut chac, mo_ho): (Vec<_>, Vec<_>) =
                tat_ca.into_iter().partition(|s| s.do_chac == DoChac::Chac);
            for s in mo_ho {
                kq.cho_xet.push(ChoXet {
                    pham_vi: s.pham_vi.clone(),
                    goc: s.goc.clone(),
                    ung_vien: vec![s.thay_bang.clone()],
                    loai: s.loai,
                    ly_do: s.ly_do.clone(),
                    bang_chung: BangChung::Khong,
                });
            }
            // Vị trí của phần để dành tính trên `chu` trước khi áp phần chắc.
            // Áp xong là lệch hết, nên phải dời lại.
            let truoc = chu.clone();
            chu = ghi_nhan(&chu, &mut chac, &mut kq);
            doi_vi_tri(&truoc, &chu, &mut kq.cho_xet);
        }

        // Tầng 5 — tiếng sai cấu tạo.
        if tc.am_tiet_sai {
            kq.cho_xet.extend(self.soat_am_tiet(&chu));
        }

        kq.chu = chu;
        kq
    }

    /// Tìm những tiếng vừa **không có trong từ điển** vừa **sai cấu tạo**.
    ///
    /// Phải trượt cả hai phép kiểm mới bị bắt, và thứ tự ấy quan trọng. Từ điển
    /// đi trước vì nó phủ được thứ mà bảng vần không bao giờ phủ nổi: từ mượn
    /// viết theo âm Việt (`bêtông`, `micrô`, `pittông`, `rađa`), tên riêng, từ
    /// địa phương. Bảng vần đi sau để đỡ cho những gì từ điển thiếu.
    ///
    /// Chỉ dùng một trong hai là hỏng theo hai kiểu khác nhau: chỉ bảng vần thì
    /// 1.800 từ mượn bị sửa hỏng; chỉ từ điển thì mọi chữ lạ hợp lệ đều bị bắt.
    fn soat_am_tiet(&self, chu: &str) -> Vec<ChoXet> {
        let tu = tach_tu::cat(chu);
        let mut ra = Vec::new();
        for (i, t) in tu.iter().enumerate() {
            let dang_kiem = match tach_tu::dang_tu(t.chu) {
                DangTu::TiengViet => true,
                DangTu::KhongDau => self.tuy_chon.chu_khong_dau,
                _ => false,
            };
            if !dang_kiem || tu_dien::co_am_tiet(t.chu) || am_tiet::hop_le(t.chu) {
                continue;
            }
            // Tên riêng đếm được từ chính cuốn sách. `Kông` trong `Hồng Kông`
            // không có trong từ điển và phạm luật chính tả (`k` không đứng
            // trước `ô`), nhưng nó là tên phiên âm chứ không phải lỗi.
            if self.ten_rieng.contains(&t.chu.to_lowercase()) {
                continue;
            }
            let truoc = i.checked_sub(1).map(|k| tu[k].chu);
            let sau = tu.get(i + 1).map(|x| x.chu);

            let uv_tho = ung_vien::sinh(t.chu);

            // Tách chữ dính xét **trước** phép sửa chữ, vì nó giữ nguyên từng ký
            // tự người ta đã gõ còn sửa chữ thì đoán họ định gõ gì — bằng chứng
            // khác hẳn về chất, không đem ra so bằng cùng một thước được.
            //
            // Không cần chặn thêm ở đây: [`tu_dien::tach_dinh`] đã đòi mảnh sau
            // mở đầu bằng phụ âm, và chính luật ấy loại hết những ca lẽ ra phải
            // sửa như một tiếng (`phảii`, `Huoàng`, `khuyếch`). Từng thử chặn
            // bằng "sửa được thành một tiếng thì đừng tách", nhưng luật ấy loại
            // nhầm cả ca đúng: `Phúlần` xoá chữ `l` ra `Phuần`, một tiếng hợp lệ.
            let tach_duoc: Vec<String> =
                tu_dien::tach_dinh(t.chu).iter().map(|x| chen_khoang_trang(t.chu, x)).collect();

            // Cách tách **dứt khoát** thì áp ngay, không hỏi ai. Cần cả hai vế:
            // cách chia mạnh (mảnh sau mở đầu bằng phụ âm), và hoặc là duy nhất
            // hoặc là dựng lại nguyên một từ ghép có thật (`erằng` → `e rằng`).
            let chac = tach_duoc.first().is_some_and(|top| {
                let thap = top.to_lowercase();
                tu_dien::tach_manh(&thap) && (tach_duoc.len() == 1 || tu_dien::co_tu_ghep(&thap))
            });
            if chac {
                ra.push(ChoXet {
                    pham_vi: t.dau..t.cuoi,
                    goc: t.chu.to_string(),
                    ly_do: format!("`{}` là hai tiếng dính liền — `{}`", t.chu, tach_duoc[0]),
                    ung_vien: tach_duoc,
                    loai: Loai::DinhChu,
                    bang_chung: BangChung::TachDinh,
                });
                continue;
            }

            // Chưa dứt khoát thì xếp **cách tách và cách sửa chữ chung một
            // bảng**, rồi để bằng chứng từ ghép phân định trên cả hai loại.
            //
            // Xếp hai bảng rời rồi nối lại là sai, và đã sai thật: hễ có một
            // cách tách cạnh tranh là bằng chứng từ ghép bị tắt, nên `Huoàng`
            // (có cách tách vô hại `hu oàng`) không được dùng tới `hoàng tử`
            // trong từ điển, và mô hình tự quyết ra `Hoang` — mất dấu thanh.
            let co_tach = !tach_duoc.is_empty();
            let so_dau = t
                .chu
                .chars()
                .filter(|&c| am_tiet::bo_thanh(c).1 != am_tiet::NGANG)
                .count();
            let (ung_vien, bang_chung) = xep_hang_ung_vien(
                t.chu,
                uv_tho,
                tach_duoc,
                so_dau,
                truoc,
                sau,
                &self.cum_ten_rieng,
                &self.tu_dung,
                self.bang_typo.tra(t.chu),
            );
            if ung_vien.is_empty() {
                continue;
            }
            let ly_do = match bang_chung {
                BangChung::Typo => format!(
                    "`{}` là typo hay gặp, bảng chỉ ghi đúng một cách sửa — `{}`",
                    t.chu, ung_vien[0]
                ),
                BangChung::CumTenRieng => format!(
                    "`{}` không có trong từ điển; `{}` dựng lại một cụm lặp lại                      nhiều lần trong chính cuốn sách này",
                    t.chu, ung_vien[0]
                ),
                BangChung::TuGhep => format!(
                    "`{}` không có trong từ điển; `{}` ghép với chữ bên cạnh thành từ có thật",
                    t.chu, ung_vien[0]
                ),
                BangChung::DauPhu => format!(
                    "`{}` chỉ thiếu một dấu phụ — `{}`, và không có cách sửa nào rẻ bằng",
                    t.chu, ung_vien[0]
                ),
                BangChung::TachDinh => format!("`{}` là hai tiếng dính liền", t.chu),
                BangChung::Khong if co_tach => {
                    format!("`{}` có thể là chữ dính, cũng có thể là lỗi gõ", t.chu)
                }
                BangChung::Khong => format!("`{}` không có trong từ điển và sai cấu tạo", t.chu),
            };
            ra.push(ChoXet {
                pham_vi: t.dau..t.cuoi,
                goc: t.chu.to_string(),
                loai: nhan_theo(Loai::AmTietSai, &ung_vien[0]),
                ung_vien,
                ly_do,
                bang_chung,
            });
        }
        ra
    }

    /// Nhờ mô hình chọn giữa bản gốc và các ứng viên, rồi áp cái thắng.
    ///
    /// Bản gốc **luôn được chấm cùng** và được cộng thêm ngưỡng — nói cách khác
    /// bản gốc thắng khi hoà. Đây là chỗ chặn cuối: mô hình nhỏ chấm hai câu
    /// gần nhau thường lệch nhau chút đỉnh theo hướng ngẫu nhiên, và mỗi lần
    /// thua sít sao mà vẫn đổi là một chữ của tác giả bị thay không lý do.
    /// `ghi` nhận một dòng mô tả cho **mỗi** chỗ mô hình xét — cả chỗ nó chọn
    /// đổi lẫn chỗ nó để nguyên, kèm chênh lệch điểm. Chỗ để nguyên quan trọng
    /// ngang chỗ đổi: nhìn vào đấy mới biết ngưỡng đang đặt quá chặt hay quá
    /// lỏng, mà không nhìn được thì cái ngưỡng ấy chỉ là một con số bịa.
    pub fn quyet_bang_mo_hinh(
        &self,
        kq: &mut KetQua,
        mo_hinh: &dyn ChamDiem,
        ngu_canh: &NguCanh,
        ghi: &mut dyn FnMut(bool, String),
    ) {
        if kq.cho_xet.is_empty() {
            return;
        }
        let goc = kq.chu.clone();
        let mut chon: Vec<SuaDoi> = Vec::new();

        for cx in kq.cho_xet.iter() {
            // Bằng chứng dứt khoát **thắng điểm số**, và bỏ qua luôn lượt chấm.
            // Từ điển nói `chúng ta` là một từ còn `chừ ta` thì không; bảng giá
            // nói `Duơng` → `Dương` chỉ thêm một dấu phụ mà không đụng vào chữ
            // nào — cả hai đều là sự thật về tiếng Việt, không phải ước lượng.
            // Hỏi mô hình ở đây chỉ tạo cơ hội cho nó phủ quyết sai: đo được là
            // nó chọn sai khoảng 40% số ca từ ghép, và trên `Hỏa Duơng Tộc` thì
            // nó đổi thành `Hỏa Vương Tộc` — đổi tên chứ không phải sửa chính tả.
            if let Some(vi_sao) = cx.bang_chung.mo_ta() {
                ghi(true, format!("`{}` → `{}` ({vi_sao})", cx.goc, cx.ung_vien[0]));
                chon.push(SuaDoi::moi(
                    cx.pham_vi.clone(),
                    cx.goc.clone(),
                    cx.ung_vien[0].clone(),
                    cx.loai,
                    DoChac::KhaChac,
                    cx.ly_do.clone(),
                ));
                continue;
            }
            // Hai lối chấm cần hai cửa sổ khác nhau, nên dựng sẵn cửa sổ rồi mới
            // vào vòng ứng viên — cùng một chỗ sửa thì bản gốc và mọi ứng viên
            // phải được chấm trên **đúng một** cửa sổ, không thì đem so hai điểm
            // của hai ngữ cảnh khác nhau.
            let cua = match self.tuy_chon.kieu_cham {
                // Chấm trên **câu chứa chỗ sửa**, không phải cả đoạn. Hai lý do:
                // mỗi ứng viên là một lượt chạy mô hình nên đoạn dài đắt gấp bội,
                // và điểm trung bình mỗi token của một đoạn dài bị phần không đổi
                // pha loãng đến mức hai ứng viên gần như bằng nhau — đúng cái ta
                // cần phân biệt thì lại bị làm mờ.
                KieuCham::CaCau => {
                    let cs = cau_chua(&goc, &cx.pham_vi);
                    let trong = cx.pham_vi.start - cs.start..cx.pham_vi.end - cs.start;
                    Cua::CaCau { nen: goc[cs].to_string(), trong }
                }
                KieuCham::ChoTrong => {
                    let (truoc, sau) = cua_so_cho_trong(&goc, ngu_canh, &cx.pham_vi);
                    Cua::ChoTrong { truoc, sau }
                }
            };
            let cham = |thay: &str| match &cua {
                Cua::CaCau { nen, trong } => mo_hinh.cham(&thay_mot_cho(nen, trong, thay)),
                Cua::ChoTrong { truoc, sau } => mo_hinh.cham_cho_trong(truoc, thay, sau),
            };
            let diem_goc = cham(&cx.goc);

            let mut tot: Option<(f32, &String)> = None;
            let mut diem_dau_bang = f32::NEG_INFINITY;
            for (i, uv) in cx.ung_vien.iter().enumerate() {
                let d = cham(uv);
                if i == 0 {
                    diem_dau_bang = d;
                }
                if tot.is_none_or(|(dt, _)| d > dt) {
                    tot = Some((d, uv));
                }
            }
            let Some((mut diem, mut uv)) = tot else { continue };

            // **Luật ngôn ngữ là mặc định; mô hình chỉ được lật ngược khi hơn
            // rõ.** Ứng viên đầu bảng do tầng luật xếp — cấu tạo âm tiết, giá
            // phép sửa, bằng chứng từ ghép — và nó chỉ nhường khi mô hình chấm
            // một ứng viên khác hơn hẳn nó, chứ không phải hơn bản gốc.
            //
            // So với bản gốc là so với một chuỗi vô nghĩa, nên ứng viên nào
            // cũng thắng và việc chọn rơi hết vào chênh lệch nhỏ giữa các ứng
            // viên — tức là vào nhiễu của mô hình. Đo được: `nòoài` xếp `ngoài`
            // đầu bảng mà mô hình chọn `ngoai`, mất dấu thanh.
            if !std::ptr::eq(uv, &cx.ung_vien[0])
                && diem - diem_dau_bang <= self.tuy_chon.nguong_mo_hinh
            {
                ghi(
                    false,
                    format!(
                        "giữ thứ tự của luật: `{}` (mô hình nghiêng về `{uv}`, chỉ hơn {:+.3})",
                        cx.ung_vien[0],
                        diem - diem_dau_bang
                    ),
                );
                uv = &cx.ung_vien[0];
                diem = diem_dau_bang;
            }
            let hon = diem - diem_goc;
            if hon <= self.tuy_chon.nguong_mo_hinh {
                ghi(
                    false,
                    format!(
                        "giữ `{}` — ứng viên khá nhất `{uv}` chỉ hơn {hon:+.3}, dưới ngưỡng {:.2}",
                        cx.goc, self.tuy_chon.nguong_mo_hinh
                    ),
                );
                continue;
            }
            ghi(true, format!("`{}` → `{uv}` (hơn {hon:+.3})", cx.goc));
            chon.push(SuaDoi::moi(
                cx.pham_vi.clone(),
                cx.goc.clone(),
                uv.clone(),
                // Nhãn theo **cách sửa được chọn**, không theo phỏng đoán lúc
                // dò: một chỗ ngờ có thể mang cả ứng viên tách chữ lẫn ứng viên
                // sửa chữ, và người đọc báo cáo cần biết cái nào đã thắng.
                nhan_theo(cx.loai, uv),
                DoChac::NgoVuc,
                format!("{} — mô hình chấm hơn {hon:+.2}", cx.ly_do),
            ));
        }
        kq.cho_xet.clear();
        let moi = ghi_nhan(&kq.chu.clone(), &mut chon, kq);
        kq.chu = moi;
    }

    /// Khi không có mô hình: chỉ áp những chỗ có **đúng một** ứng viên.
    ///
    /// Nhiều hơn một ứng viên nghĩa là bộ dò không phân được, mà không phân
    /// được thì đoán bừa. Để nguyên và ghi vào báo cáo phần "chưa sửa".
    pub fn quyet_khong_mo_hinh(&self, kq: &mut KetQua) {
        let mut chon: Vec<SuaDoi> = Vec::new();
        let mut con_lai = Vec::new();
        for cx in std::mem::take(&mut kq.cho_xet) {
            if cx.bang_chung != BangChung::Khong {
                chon.push(SuaDoi::moi(
                    cx.pham_vi.clone(),
                    cx.goc.clone(),
                    cx.ung_vien[0].clone(),
                    cx.loai,
                    DoChac::KhaChac,
                    cx.ly_do.clone(),
                ));
                continue;
            }
            if cx.ung_vien.len() == 1 && cx.loai == Loai::AmTietSai {
                chon.push(SuaDoi::moi(
                    cx.pham_vi.clone(),
                    cx.goc.clone(),
                    cx.ung_vien[0].clone(),
                    cx.loai,
                    DoChac::KhaChac,
                    format!("{} — chỉ có một cách sửa", cx.ly_do),
                ));
            } else {
                con_lai.push(cx);
            }
        }
        let moi = ghi_nhan(&kq.chu.clone(), &mut chon, kq);
        kq.chu = moi;
        kq.cho_xet = con_lai;
    }
}

/// Cửa sổ văn bản dùng để chấm một chỗ sửa, theo lối đang chọn.
enum Cua {
    /// Câu chứa chỗ sửa, kèm vị trí chỗ sửa **trong câu ấy**.
    CaCau { nen: String, trong: Range<usize> },
    /// Phần đứng trước và phần đứng sau chỗ trống.
    ChoTrong { truoc: String, sau: String },
}

/// Số câu lấy làm ngữ cảnh mỗi bên chỗ trống.
///
/// Hai vì đoạn thoại trong tiểu thuyết rất ngắn — một câu mỗi bên thì `— Ừ.`
/// chẳng cho thêm chữ nào. **Chưa đo riêng** ảnh hưởng của chính con số này; cái
/// đã đo là độ dài phần đuôi được chấm, xem [`TOI_DA_SAU`] — và trong hai chặn
/// thì chặn ấy mới là chặn thật sự bó phần đứng sau lại.
const SO_CAU_NGU_CANH: usize = 2;

/// Chặn trên cho phần đứng trước, tính bằng byte.
///
/// Để rộng vì phần này rẻ: nó giống nhau ở mọi ứng viên của cùng một chỗ sửa nên
/// bộ nhớ đệm KV của mô hình giữ lại được, mỗi chỗ chỉ nạp một lần thay vì một
/// lần cho mỗi ứng viên.
const TOI_DA_TRUOC: usize = 700;

/// Chặn trên cho phần đứng sau, tính bằng byte.
///
/// Đo trên 12 ca bẫy lấy từ tập 4 Harry Potter (`mohinh/examples/so_loi_cham.rs`),
/// chỉ đổi đúng con số này: 60 byte → 5/12 ca đúng, 180 → 6/12, 400 → 7/12. Đơn
/// điệu, nên lấy đầu trên.
///
/// **Ngược hẳn dự đoán ban đầu**, và chỗ này đáng ghi lại vì nó nghe rất hợp lý:
/// đuôi nằm ở mẫu số của điểm, nên đuôi dài thì chênh lệch giữa hai ứng viên bị
/// chia cho cả cái đuôi giống nhau ấy. Pha loãng có thật — biên độ hơn-kém trung
/// vị tụt từ 0,076 xuống 0,059 — nhưng nó **nén đúng chỗ đáng nén**: phần bị nén
/// là mấy ca mô hình tự tin sai (`zợi` → `sợi` từ +0,504 xuống −0,024), còn phép
/// sửa thật thì gần như không đổi. Đừng cắt ngắn lại cho "đỡ loãng".
///
/// Với mô hình chỉ đọc xuôi thì đây cũng là **cách duy nhất** ngữ cảnh đứng sau
/// có tác dụng: nó không được nhìn trước, nên chỉ biết đến hai câu tiếp theo qua
/// việc những câu ấy trôi chảy hay gượng gạo sau chữ vừa điền vào.
const TOI_DA_SAU: usize = 400;

/// Dựng cửa sổ cho lối điền chỗ trống: (phần đứng trước, phần đứng sau).
///
/// Ghép đoạn kề bên vào rồi mới cắt theo câu, thay vì cắt trong đoạn rồi ghép:
/// một đoạn thoại một câu thì "hai câu trước" phải lấn sang đoạn trên mới có,
/// và ranh giới đoạn cũng là một ranh giới câu nên cách ghép này tự làm đúng.
fn cua_so_cho_trong(doan: &str, nc: &NguCanh, r: &Range<usize>) -> (String, String) {
    let mut toan = String::with_capacity(nc.truoc.len() + doan.len() + nc.sau.len() + 2);
    toan.push_str(nc.truoc);
    if !nc.truoc.is_empty() {
        toan.push('\n');
    }
    let lech = toan.len();
    toan.push_str(doan);
    if !nc.sau.is_empty() {
        toan.push('\n');
        toan.push_str(nc.sau);
    }
    let (dau_trong, cuoi_trong) = (lech + r.start, lech + r.end);

    let mut dau = lui_cau(&toan, dau_trong, SO_CAU_NGU_CANH);
    if dau_trong - dau > TOI_DA_TRUOC {
        dau = dau_trong - TOI_DA_TRUOC;
        while !toan.is_char_boundary(dau) {
            dau += 1;
        }
    }
    let mut cuoi = tien_cau(&toan, cuoi_trong, SO_CAU_NGU_CANH);
    if cuoi - cuoi_trong > TOI_DA_SAU {
        cuoi = cuoi_trong + TOI_DA_SAU;
        while !toan.is_char_boundary(cuoi) {
            cuoi -= 1;
        }
    }
    (toan[dau..dau_trong].to_string(), toan[cuoi_trong..cuoi].to_string())
}

/// Ký tự có thể là ranh giới câu.
///
/// Ba chấm là **một ký tự** `…` trong sách EPUB tiếng Việt (tầng chuẩn hoá gom
/// `...` về nó), nên phải xét theo ký tự chứ không theo byte.
fn ket_cau(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '…' | '\n')
}

/// Ký tự được phép nằm giữa dấu kết câu và chữ đầu câu sau.
fn sau_ket_cau(c: char) -> bool {
    c.is_whitespace() || matches!(c, '”' | '"' | '»' | '’' | '\'' | ')')
}

/// Dấu đóng có thể dính ngay sau dấu kết câu.
fn dau_dong(c: char) -> bool {
    matches!(c, '”' | '"' | '»' | '’')
}

/// Lùi `so_cau` ranh giới câu từ vị trí `tu`, trả về chỗ bắt đầu chữ.
///
/// Dấu chấm phải có khoảng trắng hoặc dấu đóng theo sau mới tính là kết câu —
/// không thì `12.000` và `10:30` bị cắt làm hai, và cửa sổ ngữ cảnh chỉ còn ba
/// chữ số. Xuống dòng thì luôn tính, vì ranh giới đoạn mạnh hơn ranh giới câu.
///
/// Một **cụm** dấu tính là **một** ranh giới. Cuối đoạn văn thì dấu chấm và dấu
/// xuống dòng đứng liền nhau, và đếm hai thì "hai câu ngữ cảnh" thu về đúng một
/// dấu ngắt đoạn — tức là chẳng thêm câu nào. Đây là chỗ dễ sai mà khó thấy: cửa
/// sổ vẫn dựng ra, vẫn chạy, chỉ ngắn hơn ý định.
fn lui_cau(s: &str, tu: usize, so_cau: usize) -> usize {
    let mut con = so_cau;
    let mut ke_sau: Option<char> = None;
    // Ký tự vừa xét (tức ký tự đứng **sau** `c`) có thuộc cụm ranh giới không.
    let mut trong_cum = false;
    for (i, c) in s[..tu].char_indices().rev() {
        let la_ranh = ket_cau(c) && (c == '\n' || ke_sau.is_none_or(sau_ket_cau));
        if la_ranh && !trong_cum {
            con -= 1;
            if con == 0 {
                // Bỏ khoảng trắng và dấu đóng còn sót ở đầu, để cửa sổ mở đầu
                // bằng chữ chứ không bằng một dấu ngoặc mồ côi.
                let mut dau = i + c.len_utf8();
                while let Some(k) = s[dau..tu].chars().next() {
                    if sau_ket_cau(k) {
                        dau += k.len_utf8();
                    } else {
                        break;
                    }
                }
                return dau;
            }
        }
        trong_cum = la_ranh || (trong_cum && sau_ket_cau(c));
        ke_sau = Some(c);
    }
    0
}

/// Tiến `so_cau` ranh giới câu từ vị trí `tu`, trả về chỗ kết thúc.
fn tien_cau(s: &str, tu: usize, so_cau: usize) -> usize {
    let mut con = so_cau;
    let mut vt = tu;
    let mut it = s[tu..].char_indices().peekable();
    while let Some((i, c)) = it.next() {
        vt = tu + i + c.len_utf8();
        if !ket_cau(c) {
            continue;
        }
        if c != '\n' && it.peek().is_some_and(|&(_, k)| !sau_ket_cau(k)) {
            continue;
        }
        // Dấu đóng dính ngay sau dấu chấm thuộc về câu này: `.”` chứ không phải
        // `.` rồi bỏ lại `”` cho câu sau.
        while let Some(&(j, k)) = it.peek() {
            if !dau_dong(k) {
                break;
            }
            vt = tu + j + k.len_utf8();
            it.next();
        }
        con -= 1;
        if con == 0 {
            return vt;
        }
        // Nuốt nốt khoảng trắng của cụm này — cùng lý do như [`lui_cau`].
        while it.peek().is_some_and(|&(_, k)| k.is_whitespace()) {
            it.next();
        }
    }
    vt
}

/// Khoảng của câu chứa `r`.
///
/// Ranh giới câu nhận bằng `.`, `!`, `?`, `…` có khoảng trắng theo sau — cách
/// nhận thô nhưng ở đây không cần chính xác, chỉ cần **ổn định**: cùng một chỗ
/// sửa thì bản gốc và mọi ứng viên phải được cắt ra cùng một cửa sổ, không thì
/// đem so hai điểm của hai câu dài ngắn khác nhau.
///
/// Có chặn trên: câu dài quá thì cắt còn khoảng 400 byte quanh chỗ sửa, vì
/// sách dịch có những "câu" cả nghìn chữ không một dấu chấm.
fn cau_chua(chu: &str, r: &Range<usize>) -> Range<usize> {
    const TOI_DA: usize = 400;
    let mut dau = 0usize;
    let b = chu.as_bytes();
    for i in (0..r.start).rev() {
        if matches!(b[i], b'.' | b'!' | b'?') && b.get(i + 1) == Some(&b' ') {
            dau = i + 2;
            break;
        }
    }
    let mut cuoi = chu.len();
    for i in r.end..b.len() {
        if matches!(b[i], b'.' | b'!' | b'?') {
            cuoi = (i + 1).min(chu.len());
            break;
        }
    }
    if cuoi - dau > TOI_DA {
        dau = dau.max(r.start.saturating_sub(TOI_DA / 2));
        cuoi = cuoi.min(r.end + TOI_DA / 2);
        while !chu.is_char_boundary(dau) {
            dau += 1;
        }
        while !chu.is_char_boundary(cuoi) {
            cuoi -= 1;
        }
    }
    dau..cuoi
}

/// Sinh ứng viên rồi xếp lại theo bằng chứng **từ ghép**.
///
/// Phép sinh xếp hạng theo "khác bản gốc ít nhất", tức là theo hình dạng lỗi
/// gõ. Nhưng hình dạng không phân được những ứng viên cùng giá, mà chuyện ấy
/// xảy ra liên tục: `chúg` cho ra `chúng`, `chừ`, `chú`, `chug`… đều cách bản
/// gốc một bước. Ở đó thì hàng xóm quyết định — `chúng ta` có trong từ điển,
/// `chừ ta` thì không.
///
/// Đo trên một cuốn sách: mô hình ngôn ngữ 9 tỷ tham số chọn sai khoảng 40% số
/// ca thuộc loại này, và phần lớn ca sai đều là ca mà từ ghép phân được ngay.
/// Nên bằng chứng từ ghép đặt **trước** mô hình, không phải sau.
fn xep_hang_ung_vien(
    goc: &str,
    mut uv: Vec<ung_vien::UngVien>,
    tach: Vec<String>,
    so_dau_thanh: usize,
    truoc: Option<&str>,
    sau: Option<&str>,
    cum_ten_rieng: &std::collections::HashSet<String>,
    tu_dung: &std::collections::HashSet<String>,
    typo: Option<&[String]>,
) -> (Vec<String>, BangChung) {
    // Giá của một cách tách.
    //
    // **Rẻ nhất** khi nó đáng tin: nó không đổi một ký tự nào, chỉ thêm khoảng
    // trắng, nên gần bản gốc hơn mọi phép sửa chữ (phép rẻ nhất trong số ấy là
    // đảo hai chữ, giá 4).
    const GIA_TACH: u32 = 3;
    // **Đắt hơn mọi phép sửa chữ** khi nó đáng ngờ. Cách tách yếu — mảnh sau mở
    // đầu bằng nguyên âm — mà đem cho giá rẻ thì `phảii` cho ra `phả ii` đứng
    // trên `phải`, vì từ điển có cả mục `ii`.
    const GIA_TACH_YEU: u32 = 9;

    uv.extend(tach.into_iter().map(|chu| {
        let thap = chu.to_lowercase();
        // Cách tách yếu vẫn được coi là đáng tin khi chuỗi gốc mang **từ hai
        // dấu thanh trở lên**: một âm tiết tiếng Việt chỉ mang được một dấu
        // thanh, nên chuỗi ấy chắc chắn không phải một tiếng và phải tách.
        //
        // Đây là chỗ phân `ngồiở` (hai dấu → `ngồi ở`) với `phảii` (một dấu →
        // `phải`). Cả hai đều có mảnh sau mở đầu bằng nguyên âm.
        let dang_tin = tu_dien::tach_manh(&thap) || so_dau_thanh > 1;
        let gia = if dang_tin { GIA_TACH } else { GIA_TACH_YEU };
        ung_vien::UngVien { chu, gia }
    }));
    if uv.is_empty() {
        return (Vec::new(), BangChung::Khong);
    }
    // Ứng viên không có trong từ điển thì bỏ hẳn: sửa một chữ không tồn tại
    // thành một chữ khác cũng không tồn tại là đổi lỗi này lấy lỗi kia. Nhưng
    // chỉ bỏ khi còn lại thứ gì đó — từ điển không phủ hết tên riêng.
    let co_trong_tu_dien = uv.iter().any(|u| tu_dien::ung_vien_co_that(&u.chu));
    if co_trong_tu_dien {
        uv.retain(|u| tu_dien::ung_vien_co_that(&u.chu));
    }
    // Bốn tiêu chí, và **thứ tự này là toàn bộ vấn đề**: giữ được phụ âm đầu, rồi
    // từ ghép trong từ điển phổ thông, rồi từ điển riêng của chính cuốn sách, rồi
    // mới tới giá.
    //
    // **Từ điển riêng đứng cuối trong ba bảng**, và nó phải ở đó. Nó là thống kê
    // cặp tiếng đi cạnh nhau, không phải từ điển do người biên soạn — nên với chữ
    // thường nó thua `tu-ghep.txt` về chất. Đo trên tập 4 Harry Potter khi đặt nó
    // trên: `chúg tôi` ra `chủ` (sách đầy `bà chủ`, `ông chủ`), `biết điề` ra `đi`,
    // `bỏ trốngm` ra `trong`, `Thấy ẩnh` ra `anh`, `câi chuyện` ra `cái` — năm chỗ
    // hỏng, và cả năm đều do nó quyết. Việc của nó là giải cái mà **cả hai** tầng
    // trên chịu: `Hỏa Duơng Tộc` thì `tu-ghep.txt` không có `hỏa dương` nên nó im,
    // và lúc ấy bảng riêng mới lên tiếng.
    //
    // Đặt bảng riêng lên **trên cùng** thì hỏng nặng hơn nữa: trên Phàm Nhân Tu
    // Tiên số phép thay phụ âm đầu vọt từ 25 lên 160, trong đó 152 do chính nó
    // quyết — `băt` ra `mặt`, `căt` ra `bắt`, `ớc` ra `có` ở cả trăm chỗ. Lý do là
    // bảng **quá lớn** (63.964 cụm), nên gần như cặp tiếng Việt hợp lý nào cũng nằm
    // trong đó và nó quyết theo "cặp nào có trong bảng" chứ không theo "cách sửa
    // nào ít động vào chữ nhất".
    //
    // Đổi lại không mất gì của ca `Thiên Yyên Thành`: `Yên` và `Uyên` đều mở đầu
    // bằng nguyên âm nên luật giữ phụ âm đầu tự khắc im lặng, và từ điển riêng vẫn
    // là tầng quyết — xem [`giu_phu_am_dau`].
    //
    // **Giữ phụ âm đầu đứng trên hết**, kể cả trên hai bảng tra. Lỗi trong ebook
    // tiếng Việt gần như toàn bộ là lẫn dấu phụ, lẫn dấu thanh, thừa hụt một phím —
    // còn bấm hẳn sang một phụ âm khác thì hiếm. Đo trên Phàm Nhân Tu Tiên: trong
    // 108 phép sửa có thay phụ âm đầu, **71 do tầng từ ghép quyết** và 13 do tầng
    // cụm, chứ không phải do giá — nên đặt luật này dưới hai bảng tra là đặt cho có.
    // Chúng cho ra `Măc` → `Hắc`, `Côc` → `Mộc`, `Đươc` → `Nước`, `biêt` → `diệt`:
    // tra đúng bảng, sai chữ.
    //
    // Cụm phải đứng trên từ điển. `tu-ghep.txt` có `vương tộc` mà không có
    // `Hỏa Dương Tộc`, nên `Hỏa Duơng Tộc` bị sửa thành `Hỏa Vương Tộc` — đúng từ
    // điển, sai sách, và đổi luôn tên một thế lực trong truyện. Cuốn sách biết rõ
    // hơn từ điển về tên riêng của chính nó: `hỏa dương` gặp hàng chục lần trong
    // đó, `hỏa vương` không lần nào.
    // **Bảng typo đứng trên hết.** Nó là quan sát trực tiếp — `khôgn` gặp 153 lần
    // và lần nào cũng là `không` — còn mọi tầng dưới đều đang suy: suy từ bàn
    // phím, từ từ điển, từ thống kê của cuốn sách.
    //
    // Mục **nhiều đáp án** thì không quyết gì, nó chỉ thu hẹp danh sách xuống đúng
    // những chữ đã thật sự gặp, rồi các tầng dưới chọn tiếp trong đó. Xem
    // [`crate::typo`] về vì sao phải giữ vế ấy thay vì lấy đáp án đông nhất.
    let khop_typo: Vec<usize> = uv
        .iter()
        .map(|u| {
            let thap = u.chu.to_lowercase();
            usize::from(typo.is_some_and(|d| d.iter().any(|x| *x == thap)))
        })
        .collect();
    let ten: Vec<usize> =
        uv.iter().map(|u| khop_cum(cum_ten_rieng, truoc, &u.chu, sau)).collect();
    let giu: Vec<usize> = uv.iter().map(|u| usize::from(giu_phu_am_dau(goc, &u.chu))).collect();
    let khop: Vec<usize> =
        uv.iter().map(|u| tu_dien::khop_hang_xom(truoc, &u.chu, sau)).collect();
    let mut ghep: Vec<Xep> = khop_typo
        .into_iter()
        .zip(ten)
        .zip(giu)
        .zip(khop)
        .zip(uv)
        .map(|((((p, t), g), k), u)| Xep { typo: p, ten: t, giu: g, khop: k, uv: u })
        .collect();
    ghep.sort_by_key(|x| {
        (
            std::cmp::Reverse(x.typo),
            std::cmp::Reverse(x.ten),
            std::cmp::Reverse(x.giu),
            std::cmp::Reverse(x.khop),
            x.uv.gia,
        )
    });

    // Bằng chứng **từ ghép**: ứng viên đầu bảng ghép được với hàng xóm, và
    // **thắng không hoà** — không ứng viên nào khác cùng mức khớp mà giá bằng.
    //
    // Tiêu chí đầu tiên là "đúng một ứng viên ghép được", nhưng nó hỏng ngay khi
    // phép sinh mạnh lên: `tình thuơng` có tới mấy ứng viên ghép được với
    // `tình`, mà `thương` vẫn hơn hẳn phần còn lại vì nó chỉ khác bản gốc ở dấu.
    // Đòi độc nhất thì mất luôn ca dễ nhất.
    // Bằng chứng **cụm tên riêng**, mạnh nhất trong ba loại: ứng viên đầu bảng
    // dựng lại được một cụm quen của chính cuốn sách, và thắng không hoà.
    // Bằng chứng **từ điển riêng**: ứng viên đầu bảng khớp một trong ba hạng, và
    // thắng không hoà ở chính hạng ấy.
    // Bằng chứng **typo**: bảng ghi đúng một đáp án cho chữ này, và ứng viên đầu
    // bảng chính là nó. Nhiều đáp án thì không dứt khoát — đó là cả lý do giữ vế
    // ấy trong bảng.
    let typo_chac = typo.is_some_and(|d| d.len() == 1)
        && ghep.first().is_some_and(|x| x.typo > 0);

    let cum_rieng = !typo_chac
        && ghep.first().is_some_and(|x| {
            x.ten > 0
                && ghep.get(1).is_none_or(|y| (x.typo, x.ten, x.uv.gia) != (y.typo, y.ten, y.uv.gia))
        });

    let tu_ghep = !typo_chac
        && !cum_rieng
        && ghep.first().is_some_and(|x| {
            x.khop > 0
                && ghep.get(1).is_none_or(|y| {
                    (x.typo, x.ten, x.giu, x.khop, x.uv.gia)
                        != (y.typo, y.ten, y.giu, y.khop, y.uv.gia)
                })
        });

    // Bằng chứng **dấu phụ**: ứng viên đầu bảng chỉ thêm đúng một dấu phụ, là từ
    // có thật, và rẻ hơn hẳn mọi ứng viên khác.
    //
    // Không có luật này thì một phép sửa cơ học rơi vào tay tầng đoán. Đo trên
    // Phàm Nhân Tu Tiên: `Hỏa Duơng Tộc` ra `Hỏa Vương Tộc` và `Môc` ra `Lộc` —
    // tên riêng bị đổi thành tên khác, vì `tu-ghep.txt` không chứa tên riêng nên
    // bằng chứng từ ghép không với tới, còn mô hình thì vượt ngưỡng và lật ngược.
    //
    // Ba điều kiện, thiếu cái nào cũng hỏng:
    //
    // - **Chỉ thêm một dấu phụ.** Giá 2 chỉ có một đường sinh ra, xem
    //   [`ung_vien::chi_them_mot_dau_phu`]. Nới ra tới giá 4 là gộp cả phép đảo
    //   hai chữ vào, mà đảo chữ thì thật sự có thể đoán sai.
    // - **Có trong từ điển.** Danh sách trên đây chỉ được lọc bằng từ điển khi có
    //   ít nhất một ứng viên trong đó; không có thì nó giữ tất, nên ứng viên đầu
    //   bảng có thể là một chuỗi hợp cấu tạo mà chẳng ai dùng.
    // - **Rẻ hơn hẳn.** Hoà giá nghĩa là có hai cách thêm dấu phụ đều ra từ thật,
    //   và lúc ấy đúng là phải hỏi.
    let dau_phu = !typo_chac
        && !cum_rieng
        && !tu_ghep
        && ghep.first().is_some_and(|x| {
            let u = &x.uv;
            ung_vien::chi_them_mot_dau_phu(u.gia)
                && tu_dien::ung_vien_co_that(&u.chu)
                // Và **cuốn sách phải thật sự dùng chữ ấy**. Bảng âm tiết chứa cả
                // những âm tiết hợp cấu tạo mà không ai viết — xem [`gom_tu_dung`].
                // Bảng rỗng (không ai gọi `voi_tu_dung`) thì bỏ qua phép kiểm này.
                && (tu_dung.is_empty() || tu_dung.contains(&u.chu.to_lowercase()))
                && ghep.get(1).is_none_or(|y| y.uv.gia > u.gia)
        });

    let bang_chung = match (typo_chac, cum_rieng, tu_ghep, dau_phu) {
        (true, ..) => BangChung::Typo,
        (_, true, ..) => BangChung::CumTenRieng,
        (_, _, true, _) => BangChung::TuGhep,
        (.., true) => BangChung::DauPhu,
        _ => BangChung::Khong,
    };
    (ghep.into_iter().map(|x| x.uv.chu).collect(), bang_chung)
}

/// Ứng viên có **giữ nguyên phụ âm đầu** không.
///
/// Đo được cái giá của việc bỏ luật này ra khỏi đường xếp hạng, trên Phàm Nhân Tu
/// Tiên: phép thay phụ âm đầu đi từ 27 lên 42, thời gian từ 258 giây lên 1.537
/// giây (vì tầng luật loại được ít ứng viên hơn nên mô hình phải chấm gấp rưỡi),
/// và mấy ca đã vá xong quay lại — `duợc` ra `cuộc` (vì `cuộc gọi` có trong
/// `tu-ghep.txt`), `mẹnh` ra `lệnh` còn `lẹnh` ra `mệnh`.
///
/// Chỉ tính phép **thay**: cùng số chữ mà chữ đầu là một chữ cái khác. Thêm hay bớt
/// một chữ ở đầu thì không tính, vì đó là gõ thừa hoặc gõ hụt một phím chứ không
/// phải bấm sang phím khác — và lớp lỗi ấy vừa phổ biến vừa sửa đúng: `clại` →
/// `lại`, `tbên` → `bên`, `snày` → `này`, `ắmt` → `mắt`, `ớc` → `ước`. Đo trên Phàm
/// Nhân Tu Tiên thì đó là phần lớn trong 108 phép sửa đụng tới chữ đầu.
///
/// `d`/`đ`, `u`/`ư`, `o`/`ô`/`ơ` và mọi dấu thanh trên chúng tính là **cùng một
/// chữ cái** — xem [`ung_vien::cung_chu_cai`]. Đó là nhóm người gõ lẫn nhau, nên
/// đổi trong nhóm ấy không phải "thay phụ âm đầu".
///
/// Và **phụ âm** nghĩa là phụ âm: hai chữ đầu đều là nguyên âm thì đây không phải
/// chuyện phụ âm đầu, dù chúng khác chữ cái. Bỏ vế này là phá đúng lớp ca mà tầng
/// cụm tên riêng sinh ra để cứu — `Thiên Yyên Thành` ra `Thiên Yên Thành` thay vì
/// `Thiên Uyên Thành`, mà `Thiên Uyên` gặp **687 lần** trong sách còn `Thiên Yên`
/// không lần nào. `y` với `u` đều là nguyên âm, chưa từng làm phụ âm đầu tiếng Việt.
fn giu_phu_am_dau(goc: &str, moi: &str) -> bool {
    if goc.chars().count() != moi.chars().count() {
        return true;
    }
    let (Some(a), Some(b)) = (goc.chars().next(), moi.chars().next()) else {
        return true;
    };
    // Hạ chữ thường trước khi tra: bảng nguyên âm chỉ chứa chữ thường, nên
    // `la_nguyen_am('Y')` trả `false` và luật này im lặng bỏ qua mọi tên riêng —
    // đúng chỗ nó cần lên tiếng nhất.
    let nguyen_am = |c: char| {
        am_tiet::la_nguyen_am(c.to_lowercase().next().unwrap_or(c))
    };
    ung_vien::cung_chu_cai(a, b) || (nguyen_am(a) && nguyen_am(b))
}

/// Một ứng viên kèm mọi thứ hạng của nó — gộp lại cho dễ đọc, vì bốn tiêu chí xếp
/// hạng mà đi bằng tuple thì chỗ nào cũng phải đếm ngón tay.
struct Xep {
    typo: usize,
    ten: usize,
    giu: usize,
    khop: usize,
    uv: ung_vien::UngVien,
}

/// Ứng viên này ghép với hàng xóm thành mấy cụm quen của cuốn sách.
///
/// Song song với [`tu_dien::khop_hang_xom`], chỉ khác chỗ tra: bảng đếm từ chính
/// cuốn sách thay cho từ điển phổ thông. Ứng viên nhiều tiếng thì lấy tiếng đầu để
/// ghép về trước và tiếng cuối để ghép về sau, y như bên kia.
fn khop_cum(
    bang: &std::collections::HashSet<String>,
    truoc: Option<&str>,
    tieng: &str,
    sau: Option<&str>,
) -> usize {
    if bang.is_empty() {
        return 0;
    }
    let t = tieng.to_lowercase();
    let dau = t.split(' ').next().unwrap_or(&t);
    let cuoi = t.split(' ').next_back().unwrap_or(&t);
    let mut n = 0;
    if truoc.is_some_and(|p| bang.contains(&format!("{} {dau}", p.to_lowercase()))) {
        n += 1;
    }
    if sau.is_some_and(|s| bang.contains(&format!("{cuoi} {}", s.to_lowercase()))) {
        n += 1;
    }
    n
}

/// Số lần một chữ phải xuất hiện thì mới được coi là tên riêng.
///
/// Ba là chỗ đo được: lỗi gõ lặp lại y hệt ba lần trong một cuốn sách là hiếm,
/// còn tên riêng thì gặp hàng chục lần.
const LAN_DE_LA_TEN_RIENG: usize = 3;

/// Bảng đếm cụm cho [`gom_cum_sach`].
///
/// Hai tầng, và đó là chuyện bộ nhớ chứ không phải chuyện thẩm mỹ: một bộ truyện
/// 25 triệu chữ có chừng **sáu triệu** cặp tiếng liền nhau, mà phần lớn xuất hiện
/// đúng **một lần**. Giữ nguyên chữ cho tất cả thì bảng đếm ngốn hàng trăm MB. Nên
/// cặp gặp lần đầu chỉ để lại dấu vân tay 64 bit; sang lần thứ hai mới giữ chữ, và
/// số cặp gặp từ hai lần thì nhỏ hơn hẳn.
#[derive(Default)]
pub struct DemCum {
    mot_lan: std::collections::HashSet<u64>,
    tu_hai_lan: std::collections::HashMap<String, usize>,
}

impl DemCum {
    fn them(&mut self, cum: String) {
        if let Some(n) = self.tu_hai_lan.get_mut(&cum) {
            *n += 1;
            return;
        }
        let van = van_tay(&cum);
        if self.mot_lan.remove(&van) {
            self.tu_hai_lan.insert(cum, 2);
        } else {
            self.mot_lan.insert(van);
        }
    }

    /// Số cụm đang giữ chữ — để in ra cho người dùng biết bảng lớn cỡ nào.
    pub fn so_cum(&self) -> usize {
        self.tu_hai_lan.len()
    }
}

/// Gom **từ điển riêng của cuốn sách**: mọi cặp tiếng đứng liền nhau, lặp nhiều lần.
///
/// Không chỉ tên riêng. `Thiên Uyên Thành`, `Hàn Lập`, `Nguyên Anh`, `kết đan` —
/// truyện nào cũng có vốn từ riêng của nó, và vốn ấy **biết rõ hơn từ điển phổ
/// thông** về chính nó. `tu-ghep.txt` có `vương tộc` mà không có `hỏa dương`, nên
/// `Hỏa Duơng Tộc` bị sửa thành `Hỏa Vương Tộc`: đúng từ điển, sai sách.
///
/// Gom **cặp** chứ không gom cụm dài, vì lúc sửa ta có đúng một hàng xóm mỗi bên:
/// `Hỏa ??? Tộc` thì hỏi được "`hỏa dương` có quen không" và "`dương tộc` có quen
/// không". Cụm ba tiếng tự khớp thành hai cặp.
///
/// Hai thứ **không** được vào, vì bảng này đứng trên mọi tầng khác nên cái gì lọt
/// vào cũng tự bảo vệ mình trên cả cuốn sách:
///
/// - **Cụm có chữ đáng ngờ** — xem [`sach_se`].
/// - **Cụm đã có trong `tu-ghep.txt`** — thừa, vì tầng [`BangChung::TuGhep`] lo rồi;
///   và nguy, vì nó nhấc một từ ghép tầm thường lên trên luật giữ phụ âm đầu.
pub fn gom_cum_ten_rieng(van_ban: &str, dem: &mut DemCum) {
    let tu = tach_tu::cat(van_ban);
    for i in 0..tu.len().saturating_sub(1) {
        // Chỉ một khoảng trắng được xen vào. Dấu phẩy hay dấu ngoặc là hết cụm:
        // `Hỏa, Dương` không phải một cụm.
        if van_ban[tu[i].cuoi..tu[i + 1].dau] != *" " {
            continue;
        }
        if !hoa(tu[i].chu) || !hoa(tu[i + 1].chu) {
            continue;
        }
        if !sach_se(tu[i].chu) || !sach_se(tu[i + 1].chu) {
            continue;
        }
        // **Bỏ tiếng mở đầu câu.** Đầu câu thì chữ nào cũng viết hoa, nên chữ hoa ở
        // đó chẳng nói lên điều gì — không có luật này thì `Nhưng Hàn Lập` thành một
        // "cụm tên riêng" lặp hàng trăm lần trong một bộ truyện dài. Đây đúng cái
        // bẫy mà [`gom_ten_rieng`] đã vấp.
        if i == 0 || !giua_cau(van_ban, tu[i - 1].cuoi, tu[i].dau) {
            continue;
        }
        let cum = format!("{} {}", tu[i].chu.to_lowercase(), tu[i + 1].chu.to_lowercase());
        // **Cụm đã có trong từ điển phổ thông thì không lấy.** Bảng này để chứa cái
        // từ điển *không* biết. Chép lại `chúng ta`, `cuộc gọi`, `cao tầng` vào đây
        // thì vừa thừa — tầng [`BangChung::TuGhep`] vẫn lo chúng — vừa nguy, vì nó
        // nhấc một từ ghép tầm thường lên trên luật giữ phụ âm đầu.
        if !tu_dien::co_tu_ghep(&cum) {
            dem.them(cum);
        }
    }
}

/// Tiếng viết hoa — kể cả tiếng không dấu, vì tên phiên âm đầy loại ấy (`Hàn Lập`,
/// `Thiên Nam`, `Kim Đan`).
fn hoa(chu: &str) -> bool {
    chu.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Tiếng đủ **sạch** để đưa vào từ điển riêng: tiếng Việt có dấu, và **có trong từ
/// điển âm tiết**.
///
/// Đòi hẳn từ điển chứ không chỉ hợp cấu tạo, vì bảng cụm đứng trên mọi tầng khác:
/// một lỗi lọt vào đây thì nó tự bảo vệ mình trên cả cuốn sách, và không tầng nào
/// gỡ được nữa. Ngờ thì bỏ.
///
/// Cái giá là mất mấy tên phiên âm ngoài từ điển (`Kông` trong `Hồng Kông`), nhưng
/// chúng đã có [`gom_ten_rieng`] che — bảng ấy dựng theo tiêu chí khác hẳn và cho
/// chúng thoát hẳn khỏi bộ dò.
fn sach_se(chu: &str) -> bool {
    // Nhận cả tiếng **không dấu**: `cao`, `tao`, `an`, `hoa` là tiếng Việt đàng
    // hoàng. Đòi có dấu thì loại oan chúng, mà chúng chiếm phần lớn cụm thường gặp.
    // Phép kiểm thật nằm ở từ điển âm tiết ngay dưới đây — chữ nước ngoài không
    // lọt qua được nó.
    matches!(tach_tu::dang_tu(chu), DangTu::TiengViet | DangTu::KhongDau)
        && tu_dien::co_am_tiet(chu)
}

/// Dấu vân tay 64 bit của một cụm — xem [`DemCum`].
fn van_tay(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Gom **vốn từ của chính cuốn sách**: mọi tiếng tiếng Việt, viết thường hoá.
///
/// Dùng để lọc ứng viên, và nó lấp đúng lỗ hổng mà `am-tiet.txt` không lấp được:
/// bảng ấy là bảng **âm tiết hợp cấu tạo**, 9.550 mục, trong đó có cả những âm
/// tiết chẳng ai dùng. `đêu` nằm trong bảng. Nên `hắn chỉ đeu găng tay` bị sửa
/// thành `đêu găng tay` — ứng viên "có trong từ điển", chỉ thêm một dấu phụ, rẻ
/// nhất bảng, mà vẫn sai. Từ đúng là `đeo`, và cuốn sách dùng nó hàng chục lần
/// còn `đêu` thì không lần nào.
pub fn gom_tu_dung(van_ban: &str, dem: &mut std::collections::HashMap<String, usize>) {
    for t in tach_tu::cat(van_ban) {
        if tach_tu::dang_tu(t.chu) == DangTu::TiengViet {
            *dem.entry(t.chu.to_lowercase()).or_insert(0) += 1;
        }
    }
}

/// Chốt vốn từ của sách từ bảng đếm.
///
/// Cùng ngưỡng ba lần như [`LAN_DE_LA_TEN_RIENG`], và cùng lý lẽ: dưới ba lần thì
/// chưa phân được một từ tác giả dùng với một lỗi gõ lặp lại.
pub fn chot_tu_dung(
    dem: std::collections::HashMap<String, usize>,
) -> std::collections::HashSet<String> {
    dem.into_iter().filter(|(_, n)| *n >= LAN_DE_LA_TEN_RIENG).map(|(t, _)| t).collect()
}

/// Chốt danh sách cụm tên riêng từ bảng đếm.
pub fn chot_cum_ten_rieng(dem: DemCum) -> std::collections::HashSet<String> {
    dem.tu_hai_lan
        .into_iter()
        .filter(|(_, n)| *n >= LAN_DE_LA_TEN_RIENG)
        .map(|(t, _)| t)
        .collect()
}


/// Gom tên riêng từ chính cuốn sách: chữ **viết hoa**, **không có trong từ
/// điển**, và **lặp lại nhiều lần**.
///
/// Gọi cho mọi đoạn ở lượt đọc đầu, rồi đưa kết quả vào [`BoSoat::voi_ten_rieng`].
///
/// Vì sao phải đếm cả sách thay vì nhìn một chữ: `Kông` trong `Hồng Kông` và
/// `Kó` (gõ nhầm `Có`) trông giống hệt nhau nếu chỉ nhìn một chữ — cùng viết
/// hoa, cùng không có trong từ điển, cùng phạm luật `k` không đứng trước
/// nguyên âm sau. Chỉ có số lần xuất hiện phân được: tên riêng lặp lại, lỗi gõ
/// thì không.
///
/// Đòi **viết hoa** là điều kiện then chốt. Không có nó thì `khôgn` — gõ nhầm
/// `không`, lặp 153 lần trong một cuốn — cũng thành "tên riêng" và thoát hết.
pub fn gom_ten_rieng(van_ban: &str, dem: &mut std::collections::HashMap<String, usize>) {
    let tu = tach_tu::cat(van_ban);
    for (i, t) in tu.iter().enumerate() {
        if tach_tu::dang_tu(t.chu) != DangTu::TiengViet {
            continue;
        }
        if !t.chu.chars().next().is_some_and(|c| c.is_uppercase()) {
            continue;
        }
        if tu_dien::co_am_tiet(t.chu) {
            continue;
        }
        // **Chữ hoa phải nằm giữa câu**, không phải đầu câu. Đầu câu thì chữ
        // nào cũng viết hoa nên chữ hoa chẳng nói lên điều gì — mà lỗi gõ hay
        // rơi vào đầu câu đúng như mọi chỗ khác. Không có luật này thì `Chẵng`
        // (gõ nhầm `Chẳng`, lặp mấy chục lần) được xếp là tên riêng rồi thoát.
        //
        // Giữa câu thì tiếng Việt chỉ viết hoa cho tên riêng, nên tín hiệu rất
        // mạnh — đủ mạnh để giữ `Kông` trong `Hồng Kông`.
        if i == 0 || !giua_cau(van_ban, tu[i - 1].cuoi, t.dau) {
            continue;
        }
        *dem.entry(t.chu.to_lowercase()).or_insert(0) += 1;
    }
}

/// Khoảng giữa hai chữ có phải chỉ là khoảng trắng thường không.
///
/// Có dấu kết câu, dấu ngoặc kép hay xuống dòng xen vào thì chữ sau là **đầu
/// câu**, và việc nó viết hoa không nói lên điều gì.
fn giua_cau(van_ban: &str, tu_vi_tri: usize, den: usize) -> bool {
    van_ban[tu_vi_tri..den].chars().all(|c| c == ' ' || c == ',' || c == ';')
}

/// Chốt danh sách tên riêng từ bảng đếm.
pub fn chot_ten_rieng(
    dem: std::collections::HashMap<String, usize>,
) -> std::collections::HashSet<String> {
    dem.into_iter().filter(|(_, n)| *n >= LAN_DE_LA_TEN_RIENG).map(|(t, _)| t).collect()
}

/// Chèn khoảng trắng vào **chính chuỗi gốc**, theo cách chia đã tìm được.
///
/// [`tu_dien::tach_dinh`] làm việc trên bản viết thường nên kết quả của nó mất
/// hết chữ hoa. Không được lấy thẳng: chữ dính hay dính đúng ở chỗ tên riêng
/// (`mựcMinh`, `HuyềnVũ`, `LãoTạ`), mà đó cũng là chỗ chữ hoa mang thông tin.
/// Lấy thẳng thì ra `mực minh`, `Huyền vũ` — tách đúng chỗ nhưng xoá mất tên
/// người.
///
/// Nên chỉ lấy **vị trí ngắt** từ cách chia, rồi cắt trên chuỗi gốc. Như thế
/// từng ký tự giữ nguyên hình dạng người ta đã gõ, và phép sửa này thật sự chỉ
/// thêm vào mấy khoảng trắng.
fn chen_khoang_trang(goc: &str, cach_chia: &str) -> String {
    let ky_tu: Vec<char> = goc.chars().collect();
    let mut ra = String::with_capacity(goc.len() + 4);
    let mut vt = 0usize;
    for (i, manh) in cach_chia.split(' ').enumerate() {
        let n = manh.chars().count();
        if vt + n > ky_tu.len() {
            // Cách chia không khớp độ dài chuỗi gốc — không thể xảy ra, nhưng
            // thà trả về bản viết thường còn hơn cắt vào giữa chữ.
            return cach_chia.to_string();
        }
        if i > 0 {
            ra.push(' ');
        }
        ra.extend(&ky_tu[vt..vt + n]);
        vt += n;
    }
    if vt != ky_tu.len() {
        return cach_chia.to_string();
    }
    ra
}

/// Nhãn loại lỗi suy từ cách sửa đã chọn: có khoảng trắng nghĩa là đã tách chữ.
fn nhan_theo(loai: Loai, da_chon: &str) -> Loai {
    if !matches!(loai, Loai::DinhChu | Loai::AmTietSai) {
        return loai;
    }
    if da_chon.contains(' ') {
        Loai::DinhChu
    } else {
        Loai::AmTietSai
    }
}

fn thay_mot_cho(goc: &str, r: &Range<usize>, moi: &str) -> String {
    let mut s = String::with_capacity(goc.len() + moi.len());
    s.push_str(&goc[..r.start]);
    s.push_str(moi);
    s.push_str(&goc[r.end..]);
    s
}

/// Áp một loạt phép sửa, ghi những phép **thật sự áp được** vào kết quả.
///
/// Không ghi thẳng cả danh sách vào báo cáo: `ap_dung` bỏ phép chồng nhau, mà
/// báo cáo liệt kê một phép sửa không xảy ra thì người dùng đi tìm nó trong
/// sách và không thấy.
fn ghi_nhan(chu: &str, s: &mut Vec<SuaDoi>, kq: &mut KetQua) -> String {
    s.retain(|x| x.co_doi());
    if s.is_empty() {
        return chu.to_string();
    }
    let (moi, _) = ap_dung(chu, s);
    // `ap_dung` đã sắp `s` và bỏ phép chồng; dựng lại danh sách áp được bằng
    // cùng luật để báo cáo khớp đúng những gì đã xảy ra.
    let mut cuoi = 0usize;
    for x in s.iter() {
        if x.pham_vi.start < cuoi || x.pham_vi.end > chu.len() {
            continue;
        }
        cuoi = x.pham_vi.end;
        kq.da_sua.push(x.clone());
    }
    moi
}

/// Dời vị trí các chỗ-để-xét sau khi chuỗi đã đổi.
///
/// Cách làm thô nhưng đúng: tìm lại chuỗi gốc quanh vị trí cũ trong chuỗi mới.
/// Không tìm thấy thì bỏ chỗ ấy — thà bỏ sót còn hơn sửa nhầm vị trí, vì sửa
/// nhầm vị trí là cắt vào giữa một chữ khác.
fn doi_vi_tri(truoc: &str, sau: &str, cho_xet: &mut Vec<ChoXet>) {
    if truoc == sau {
        return;
    }
    cho_xet.retain_mut(|cx| {
        // Neo bằng chính chuỗi gốc, tìm chỗ gần vị trí cũ nhất.
        let mut tot: Option<usize> = None;
        let mut tu = 0usize;
        while let Some(k) = sau[tu..].find(&cx.goc) {
            let vt = tu + k;
            if tot.is_none_or(|t| {
                vt.abs_diff(cx.pham_vi.start) < t.abs_diff(cx.pham_vi.start)
            }) {
                tot = Some(vt);
            }
            tu = vt + 1;
            if tu >= sau.len() {
                break;
            }
        }
        match tot {
            Some(vt) => {
                cx.pham_vi = vt..vt + cx.goc.len();
                true
            }
            None => false,
        }
    });
}

#[cfg(test)]
mod kiem {
    use super::*;

    /// Mô hình giả: chấm điểm bằng cách đếm cụm từ trong danh sách "câu tự
    /// nhiên". Đủ để kiểm phần điều phối mà không cần nạp mô hình thật.
    struct MoHinhGia(Vec<&'static str>);
    impl ChamDiem for MoHinhGia {
        fn cham(&self, cau: &str) -> f32 {
            let thap = cau.to_lowercase();
            self.0.iter().filter(|m| thap.contains(*m)).count() as f32
        }
    }

    fn bo() -> BoSoat {
        BoSoat::moi(TuyChon::default(), Kieu::Cu)
    }

    #[test]
    fn chay_het_cac_tang_tren_mot_doan() {
        let mut kq = bo().soat("Anh ấy xử dụng máy tính , rồi   đi ra .");
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, "Anh ấy sử dụng máy tính, rồi đi ra.");
        assert!(kq.da_sua.len() >= 3, "{:?}", kq.da_sua);
    }

    #[test]
    fn van_ban_sach_thi_khong_doi_gi() {
        let v = "Anh ấy nói: “Tôi không biết.” Rồi bỏ đi, chẳng ngoái lại.";
        let mut kq = bo().soat(v);
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, v);
        assert!(kq.da_sua.is_empty(), "{:?}", kq.da_sua);
        assert!(kq.cho_xet.is_empty(), "{:?}", kq.cho_xet);
    }

    #[test]
    fn tieng_sai_nhieu_ung_vien_thi_de_lai_cho_mo_hinh() {
        let kq = bo().soat("Tình thuơng của mẹ");
        assert_eq!(kq.cho_xet.len(), 1);
        assert!(kq.cho_xet[0].ung_vien.contains(&"thương".to_string()));
    }

    #[test]
    fn mo_hinh_chon_ung_vien_dung() {
        let mh = MoHinhGia(vec!["tình thương"]);
        let mut kq = bo().soat("Tình thuơng của mẹ");
        bo().quyet_bang_mo_hinh(&mut kq, &mh, &NguCanh::default(), &mut |_, _| {});
        assert_eq!(kq.chu, "Tình thương của mẹ");
    }

    struct Deu;
    impl ChamDiem for Deu {
        fn cham(&self, _: &str) -> f32 {
            1.0
        }
    }

    #[test]
    fn mo_hinh_thang_sit_sao_thi_khong_doi() {
        // Mô hình chấm mọi ứng viên bằng nhau — không có bằng chứng gì. Bản gốc
        // phải sống sót. Đây là cái van chặn ứng dụng tự sửa bừa.
        //
        // Câu phải chọn sao cho **không bằng chứng nào dứt khoát**: không ứng viên
        // nào ghép được với hàng xóm, và phép sửa không phải loại "chỉ thêm một
        // dấu phụ" — không thì tầng luật quyết trước và mô hình chẳng được hỏi.
        let v = "Ừ nhưmg à";
        let mut kq = bo().soat(v);
        assert_eq!(
            kq.cho_xet[0].bang_chung,
            BangChung::Khong,
            "ca này lẽ ra không có bằng chứng dứt khoát nào"
        );
        bo().quyet_bang_mo_hinh(&mut kq, &Deu, &NguCanh::default(), &mut |_, _| {});
        assert_eq!(kq.chu, v);
    }

    #[test]
    fn cum_ten_rieng_cua_sach_quyet_truoc_tu_dien_pho_thong() {
        // `Hỏa Dương Tộc` là tên một thế lực trong truyện. `tu-ghep.txt` không có
        // nó — nhưng **có** `vương tộc`, nên nếu chỉ có từ điển phổ thông thì
        // `Hỏa Duơng Tộc` ra `Hỏa Vương Tộc`: đúng từ điển, sai sách.
        let mut dem = DemCum::default();
        for _ in 0..LAN_DE_LA_TEN_RIENG {
            gom_cum_ten_rieng("Đám cao tầng Hỏa Dương Tộc không dám tới.", &mut dem);
        }
        let cum = chot_cum_ten_rieng(dem);
        assert!(cum.contains("hỏa dương"), "{cum:?}");
        assert!(cum.contains("dương tộc"), "{cum:?}");
        // Chỉ tên riêng mới vào: `cao tầng` là hai tiếng thường.
        assert!(!cum.contains("cao tầng"), "{cum:?}");

        let bo = BoSoat::moi(TuyChon::default(), Kieu::Cu).voi_cum_ten_rieng(cum);
        let mut kq = bo.soat("Đám cao tầng Hỏa Duơng Tộc không dám tới.");
        assert_eq!(kq.cho_xet[0].bang_chung, BangChung::CumTenRieng, "{:?}", kq.cho_xet);
        // `Deu` chấm mọi ứng viên bằng nhau: mô hình không có ý kiến gì, nên bài
        // kiểm này bắt đúng đường tầng tên riêng tự quyết.
        bo.quyet_bang_mo_hinh(&mut kq, &Deu, &NguCanh::default(), &mut |_, _| {});
        assert!(kq.chu.contains("Hỏa Dương Tộc"), "{}", kq.chu);

        // Không có bảng tên riêng thì luật "giữ phụ âm đầu" đỡ được — `Vương` phải
        // bấm sang một phụ âm khác, `Dương` thì không — rồi giá chốt. Bỏ luật ấy ra
        // thì `tu-ghep.txt` có `vương tộc` và cả cuốn sách ra `Hỏa Vương Tộc`; đã
        // đo và ghi trong [`giu_phu_am_dau`].
        let bo = BoSoat::moi(TuyChon::default(), Kieu::Cu);
        let kq = bo.soat("Đám cao tầng Hỏa Duơng Tộc không dám tới.");
        assert_eq!(kq.cho_xet[0].ung_vien[0], "Dương", "{:?}", kq.cho_xet[0].ung_vien);
        assert_eq!(kq.cho_xet[0].bang_chung, BangChung::DauPhu);
    }

    #[test]
    fn giu_phu_am_dau_dung_tren_ca_hai_bang_tra() {
        // `Măc` → `Mặc` chỉ thêm một dấu thanh; `Măc` → `Hắc` bấm sang một phụ âm
        // khác. Trong truyện tu tiên thì `Hắc` là tiếng đặt tên rất thường, nên cả
        // từ điển lẫn cụm của sách đều đỡ cho nó — và trước khi có luật này thì mô
        // hình chọn `Hắc`.
        assert!(giu_phu_am_dau("Măc", "Mặc"));
        assert!(!giu_phu_am_dau("Măc", "Hắc"));

        // Thêm hoặc bớt một chữ ở đầu **không** tính là thay phụ âm đầu: đó là gõ
        // thừa hoặc gõ hụt một phím, và lớp lỗi ấy sửa đúng rất nhiều.
        assert!(giu_phu_am_dau("clại", "lại"));
        assert!(giu_phu_am_dau("ớc", "ước"));
        assert!(giu_phu_am_dau("Kông", "Không"));

        // `u`/`ư` là cùng một chữ cái — đúng nhóm nguyên âm người gõ lẫn nhau.
        assert!(giu_phu_am_dau("uơng", "ương"));

        // Nhưng **`d` và `đ` thì không**: hai phụ âm khác nhau. Thêm dấu vào `d` là
        // phép sửa rẻ (giá 2), nhưng rẻ không có nghĩa là cùng chữ cái — rơi vào
        // đầu chữ thì nó vẫn là đổi phụ âm đầu.
        //
        // Đo trên Phàm Nhân Tu Tiên: gộp chúng lại thì `duợc` ra `được` ở cả hai
        // chỗ, mà cả hai đều là `dược` — một chỗ có ngay `dược tính` viết đúng ở
        // câu bên cạnh. Tách ra thì `dược` thắng vì nó giữ `d`.
        assert!(!giu_phu_am_dau("dang", "đang"));
        assert!(!giu_phu_am_dau("duợc", "được"));
        assert!(giu_phu_am_dau("duợc", "dược"));

        // Hai chữ đầu đều là **nguyên âm** thì không phải chuyện phụ âm đầu, dù
        // chúng khác chữ cái. `Thiên Yyên Thành` phải ra `Thiên Uyên Thành` — cụm
        // ấy gặp 687 lần trong sách, `Thiên Yên` không lần nào — nên `Uyên` không
        // được bị luật này đè xuống dưới `Yên`.
        assert!(giu_phu_am_dau("Yyên", "Uyên"));
        // Còn một bên là phụ âm thì tính: `ớc` → `cố` đẩy một phụ âm vào đầu chữ.
        assert!(!giu_phu_am_dau("ớc", "cố"));
    }

    #[test]
    fn dau_cau_xen_giua_thi_khong_phai_mot_cum() {
        // `Hỏa, Dương` không phải một cụm: có dấu phẩy xen vào. Và tiếng mở đầu câu
        // thì bỏ — đầu câu chữ nào cũng viết hoa nên chữ hoa ở đó chẳng nói gì.
        let mut dem = DemCum::default();
        for _ in 0..LAN_DE_LA_TEN_RIENG {
            gom_cum_ten_rieng("Hỏa, Dương đi rồi. Nhưng Hàn Lập vẫn ngồi.", &mut dem);
        }
        let cum = chot_cum_ten_rieng(dem);
        assert!(!cum.contains("hỏa dương"), "{cum:?}");
        assert!(!cum.contains("nhưng hàn"), "{cum:?}");
        assert!(cum.contains("hàn lập"), "{cum:?}");

        // Và **chữ đáng ngờ thì không được vào**: `Duơng` không có trong từ điển âm
        // tiết, nên `hỏa duơng` phải bị loại kể cả khi nó lặp lại nhiều lần. Đây là
        // cái van của cả tầng này.
        let mut dem = DemCum::default();
        for _ in 0..LAN_DE_LA_TEN_RIENG + 4 {
            gom_cum_ten_rieng("Đám cao tầng Hỏa Duơng Tộc không dám tới.", &mut dem);
        }
        let cum = chot_cum_ten_rieng(dem);
        assert!(!cum.contains("hỏa duơng"), "{cum:?}");
        assert!(!cum.contains("duơng tộc"), "{cum:?}");
    }

    #[test]
    fn cum_da_co_trong_tu_dien_pho_thong_thi_khong_lay() {
        // Từ điển riêng để chứa cái từ điển phổ thông **không** biết. Chép lại
        // `chúng ta`, `cuộc gọi` vào đây thì vừa thừa vừa nguy: nó nhấc một từ ghép
        // tầm thường lên trên luật giữ phụ âm đầu, và `đã đựoc gọi lên` lại ra
        // `đã cuộc gọi lên`.
        let mut dem = DemCum::default();
        for _ in 0..LAN_DE_LA_TEN_RIENG {
            gom_cum_ten_rieng("Một cuộc gọi tới, chúng ta nên đi Thiên Uyên ngay.", &mut dem);
        }
        let cum = chot_cum_ten_rieng(dem);
        assert!(tu_dien::co_tu_ghep("cuộc gọi") && tu_dien::co_tu_ghep("chúng ta"));
        assert!(!cum.contains("cuộc gọi"), "{cum:?}");
        assert!(!cum.contains("chúng ta"), "{cum:?}");
        // Còn cụm riêng của sách thì vẫn vào.
        assert!(cum.contains("thiên uyên"), "{cum:?}");
    }

    #[test]
    fn thieu_mot_dau_phu_thi_khong_hoi_mo_hinh() {
        // `Hỏa Duơng Tộc` — tên riêng, nên `tu-ghep.txt` không có `Hỏa Dương` hay
        // `Dương Tộc` và bằng chứng từ ghép không với tới. Nhưng `Dương` chỉ thêm
        // đúng một dấu phụ vào `u` mà không đụng vào chữ nào khác, và không cách
        // sửa nào rẻ bằng — đó là sự thật về bàn phím, không phải một ước lượng.
        //
        // Không có luật này thì chỗ ấy rơi vào tay mô hình, và đo được trên sách
        // thật là nó đổi thành `Hỏa Vương Tộc` — đổi tên nhân vật chứ không phải
        // sửa chính tả. `Deu` ở đây chấm mọi ứng viên bằng nhau, tức là mô hình
        // không có ý kiến gì, nên bài kiểm này bắt đúng đường tầng luật tự quyết.
        let mut kq = bo().soat("Ừ Duơng à");
        assert_eq!(kq.cho_xet[0].bang_chung, BangChung::DauPhu, "{:?}", kq.cho_xet);
        bo().quyet_bang_mo_hinh(&mut kq, &Deu, &NguCanh::default(), &mut |_, _| {});
        assert_eq!(kq.chu, "Ừ Dương à");

        // Và sửa được cả khi không có card đồ hoạ.
        let mut kq = bo().soat("Ừ Duơng à");
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, "Ừ Dương à");
    }

    #[test]
    fn hai_cach_them_dau_phu_deu_ra_tu_that_thi_van_phai_hoi() {
        // Luật trên đòi ứng viên đầu bảng **rẻ hơn hẳn**. Hoà giá nghĩa là có hơn
        // một cách thêm dấu phụ đều ra từ có thật, và lúc ấy bảng giá không biết
        // chọn cái nào — phải để mô hình quyết chứ không được tự áp.
        //
        // Dựng thẳng danh sách ứng viên chứ không đi qua một chuỗi thật: ca hoà giá
        // ở mức 2 có tồn tại nhưng khó dựng ra bằng một chuỗi ngắn dễ đọc, mà điều
        // cần ghim là **luật**, không phải chuỗi nào kích hoạt được nó.
        let uv = vec![
            ung_vien::UngVien { chu: "thương".into(), gia: 2 },
            ung_vien::UngVien { chu: "thường".into(), gia: 2 },
        ];
        let (_, bc) = xep_hang_ung_vien(
            "thuong",
            uv,
            Vec::new(),
            0,
            None,
            None,
            &Default::default(),
            &Default::default(),
            None,
        );
        assert_eq!(bc, BangChung::Khong);

        // Còn thắng không hoà thì mới dứt khoát.
        let uv = vec![
            ung_vien::UngVien { chu: "thương".into(), gia: 2 },
            ung_vien::UngVien { chu: "thường".into(), gia: 5 },
        ];
        let (_, bc) = xep_hang_ung_vien(
            "thuong",
            uv,
            Vec::new(),
            0,
            None,
            None,
            &Default::default(),
            &Default::default(),
            None,
        );
        assert_eq!(bc, BangChung::DauPhu);
    }

    #[test]
    fn tu_ghep_quyet_truoc_mo_hinh() {
        // `tình thương` có trong từ điển, `tình thường`/`tình thưởng` thì không.
        // Bằng chứng ấy dứt khoát nên phải áp được **kể cả khi mô hình phản
        // đối** — mô hình chấm đều nhau ở đây, tức là nó không có ý kiến gì.
        let mut kq = bo().soat("Tình thuơng của mẹ");
        assert_eq!(kq.cho_xet[0].bang_chung, BangChung::TuGhep);
        bo().quyet_bang_mo_hinh(&mut kq, &Deu, &NguCanh::default(), &mut |_, _| {});
        assert_eq!(kq.chu, "Tình thương của mẹ");
    }

    #[test]
    fn tu_ghep_sua_duoc_khi_khong_co_mo_hinh() {
        // Không có card đồ hoạ thì vẫn sửa được lớp lỗi này.
        let mut kq = bo().soat("Tình thuơng của mẹ");
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, "Tình thương của mẹ");
    }

    /// Mô hình giả cho lối điền chỗ trống. Ghi lại **chỗ trống nó nhìn thấy** để
    /// bài kiểm soi được ngữ cảnh có tới tay mô hình hay không.
    struct GiaChoTrong {
        thich: Vec<&'static str>,
        thay: std::cell::RefCell<Vec<(String, String, String)>>,
    }
    impl GiaChoTrong {
        fn moi(thich: Vec<&'static str>) -> GiaChoTrong {
            GiaChoTrong { thich, thay: Default::default() }
        }
    }
    impl ChamDiem for GiaChoTrong {
        fn cham(&self, _: &str) -> f32 {
            // Lối điền chỗ trống không được rơi về đây. Rơi về thì bài kiểm
            // dưới thành ra kiểm lối cũ mà vẫn xanh.
            panic!("lối điền chỗ trống không được gọi `cham`");
        }
        fn cham_cho_trong(&self, truoc: &str, dien: &str, sau: &str) -> f32 {
            self.thay.borrow_mut().push((truoc.into(), dien.into(), sau.into()));
            let cum = format!("{truoc}{dien}{sau}").to_lowercase();
            self.thich.iter().filter(|m| cum.contains(*m)).count() as f32
        }
    }

    fn bo_cho_trong() -> BoSoat {
        BoSoat::moi(
            TuyChon { kieu_cham: KieuCham::ChoTrong, ..TuyChon::default() },
            Kieu::Cu,
        )
    }

    #[test]
    fn loi_cho_trong_chon_duoc_ung_vien_dung() {
        // `Ừ nhưmg à` — gõ trượt phím, không hàng xóm nào ghép thành từ có thật,
        // và phép sửa không phải loại "chỉ thêm một dấu phụ". Nên không bằng chứng
        // nào quyết được và mô hình mới được hỏi. Đây là đúng lớp ca mà lối điền
        // chỗ trống nhắm tới.
        let mh = GiaChoTrong::moi(vec!["nhưng"]);
        let mut kq = bo_cho_trong().soat("Ừ nhưmg à");
        bo_cho_trong().quyet_bang_mo_hinh(&mut kq, &mh, &NguCanh::default(), &mut |_, _| {});
        assert_eq!(kq.chu, "Ừ nhưng à");
    }

    #[test]
    fn loi_cho_trong_khoet_dung_cho_va_dua_du_ngu_canh() {
        let mh = GiaChoTrong::moi(vec![]);
        let nc = NguCanh {
            truoc: "Trời đã tối. Hắn ngồi im rất lâu.",
            sau: "Rồi hắn đứng lên. Cửa mở.",
        };
        let mut kq = bo_cho_trong().soat("Ừ nhưmg à");
        bo_cho_trong().quyet_bang_mo_hinh(&mut kq, &mh, &nc, &mut |_, _| {});
        let thay = mh.thay.borrow();
        assert!(!thay.is_empty(), "mô hình không được hỏi lần nào");
        let (truoc, dien, sau) = &thay[0];
        // Chỗ trống phải đúng chữ sai, không lệch một ký tự nào.
        assert_eq!(dien, "nhưmg");
        // Ngữ cảnh phải lấn sang đoạn kề: `Ừ thuơng à` chỉ có một câu, mà lối này
        // hứa hai câu mỗi bên.
        assert!(truoc.contains("Hắn ngồi im"), "thiếu ngữ cảnh đoạn trước: {truoc:?}");
        assert!(truoc.ends_with("Ừ "), "cắt sai chỗ trống: {truoc:?}");
        assert!(sau.starts_with(" à"), "cắt sai phần đuôi: {sau:?}");
        assert!(sau.contains("Rồi hắn đứng lên"), "thiếu ngữ cảnh đoạn sau: {sau:?}");
        // Và phải **dừng** ở hai câu, không kéo cả cuốn sách vào.
        assert!(!truoc.contains("Trời đã tối"), "lấy quá hai câu: {truoc:?}");
        assert!(!sau.contains("Cửa mở"), "lấy quá hai câu: {sau:?}");
    }

    #[test]
    fn loi_cho_trong_thang_sit_sao_thi_khong_doi() {
        // Cái van an toàn phải chặn ở cả hai lối chấm: mô hình không phân được
        // thì bản gốc sống sót.
        let mh = GiaChoTrong::moi(vec![]);
        let v = "Ừ nhưmg à";
        let mut kq = bo_cho_trong().soat(v);
        bo_cho_trong().quyet_bang_mo_hinh(&mut kq, &mh, &NguCanh::default(), &mut |_, _| {});
        assert_eq!(kq.chu, v);
    }

    #[test]
    fn con_so_khong_bi_coi_la_het_cau() {
        // `12.000` và `10:30` đầy trong sách. Coi dấu chấm ấy là hết câu thì cửa
        // sổ ngữ cảnh cụt còn ba chữ số — mà đó là lúc ngữ cảnh cần nhất, vì
        // quanh con số thì chữ nào cũng lạ.
        let s = "Hắn trả 12.000 đồng cho chỗ ấy. Rồi đi.";
        let vt = s.find("đồng").unwrap();
        assert_eq!(lui_cau(s, vt, 1), 0, "cắt vào giữa con số");
    }

    #[test]
    fn cham_roi_xuong_dong_chi_la_mot_ranh_gioi() {
        // Cuối đoạn văn có cả dấu chấm lẫn dấu ngắt đoạn. Đếm hai thì xin hai câu
        // mà chỉ nhận về một dấu ngắt.
        let s = "Câu một. Câu hai.\nCâu ba.";
        let vt = s.find("ba").unwrap();
        assert_eq!(lui_cau(s, vt, 2), s.find("Câu hai").unwrap(), "{:?}", &s[..vt]);
        assert_eq!(tien_cau(s, s.find("một").unwrap(), 2), s.find('\n').unwrap());
    }

    #[test]
    fn dau_ngoac_kep_dong_thuoc_ve_cau_truoc() {
        // Thoại tiếng Việt kết bằng `.”`, và cửa sổ phải mở đầu bằng chữ chứ
        // không bằng một dấu ngoặc mồ côi.
        let s = "Hắn nói: “Tôi không biết.” Rồi bỏ đi, chẳng ngoái lại. Hết.";
        let vt = s.find("chẳng").unwrap();
        let dau = lui_cau(s, vt, 1);
        assert!(s[dau..].starts_with("Rồi bỏ đi"), "{:?}", &s[dau..]);
        let cuoi = tien_cau(s, s.find("Tôi").unwrap(), 1);
        assert_eq!(&s[..cuoi], "Hắn nói: “Tôi không biết.”");
    }

    #[test]
    fn ten_rieng_lap_lai_thi_duoc_giu_nguyen() {
        // `Kông` không có trong từ điển và phạm luật chính tả (`k` không đứng
        // trước `ô`), nhưng trong `Hồng Kông` nó là tên phiên âm chứ không phải
        // lỗi. Phân được nhờ đếm cả sách: tên riêng lặp lại, lỗi gõ thì không.
        let mut dem = std::collections::HashMap::new();
        for cau in [
            "Tôi đến Hồng Kông chơi.",
            "Ở Hồng Kông có nhiều người.",
            "Chuyến bay tới Hồng Kông bị hoãn.",
        ] {
            gom_ten_rieng(cau, &mut dem);
        }
        let ten = chot_ten_rieng(dem);
        assert!(ten.contains("kông"), "{ten:?}");

        let bo = BoSoat::moi(TuyChon::default(), Kieu::Cu).voi_ten_rieng(ten);
        let v = "Tôi đến Hồng Kông chơi.";
        let mut kq = bo.soat(v);
        bo.quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, v);
    }

    #[test]
    fn chu_hoa_dau_cau_khong_phai_ten_rieng() {
        // `Chẵng` là gõ nhầm `Chẳng`, và nó đứng đầu câu nên chữ hoa chẳng nói
        // lên điều gì. Không có luật này thì mọi lỗi gõ hay rơi vào đầu câu đều
        // được xếp là tên riêng rồi thoát hết.
        let mut dem = std::collections::HashMap::new();
        for _ in 0..5 {
            gom_ten_rieng("Chẵng ai biết. Chẵng ai hay.", &mut dem);
        }
        assert!(chot_ten_rieng(dem).is_empty());
    }

    #[test]
    fn d_va_d_gach_deu_la_tu_that_nen_khong_ai_dung_vao() {
        // `dang` (dang tay, dở dang) và `đang` (đang làm) là hai từ khác nhau,
        // cả hai đều có trong từ điển. Bộ dò chỉ xét chữ **không có** trong từ
        // điển, nên không chữ nào trong hai chữ này từng bị đem ra cân nhắc —
        // dù phép sinh ứng viên có biết đường `dang` → `đang` đi nữa.
        for v in ["Tay dang rộng, việc còn dở dang.", "Anh đang làm gì đấy?"] {
            let mut kq = bo().soat(v);
            bo().quyet_khong_mo_hinh(&mut kq);
            assert_eq!(kq.chu, v);
            assert!(kq.cho_xet.is_empty(), "{:?}", kq.cho_xet);
        }
    }

    #[test]
    fn khong_dung_vao_tu_muon_co_trong_tu_dien() {
        // `bêtông`, `micrô`, `rađa` sai cấu tạo âm tiết nhưng có trong từ điển.
        // Trước khi có tầng từ điển, cả 1.800 chữ kiểu này bị sửa hỏng.
        let v = "Cột bêtông và cái micrô cùng cái rađa";
        let mut kq = bo().soat(v);
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, v);
        assert!(kq.cho_xet.is_empty(), "{:?}", kq.cho_xet);
    }

    #[test]
    fn khong_dung_vao_ten_rieng_nuoc_ngoai() {
        // `Dumbledore`, `Voldemort` không phải tiếng Việt và cũng không mang
        // dấu — mặc định không đụng tới.
        let v = "Giáo sư Dumbledore nhìn Voldemort.";
        let mut kq = bo().soat(v);
        bo().quyet_khong_mo_hinh(&mut kq);
        assert_eq!(kq.chu, v);
    }
}
