//! Cửa sổ ứng dụng.
//!
//! Một màn hình duy nhất, chia ba phần theo đúng thứ tự người dùng đi qua: chọn
//! sách, xem lại cài đặt, rồi theo dõi nó chạy.
//!
//! Phần thứ ba chiếm chỗ nhiều nhất, và có lý do. Một cuốn truyện dài chạy hàng
//! phút, đi qua sáu giai đoạn khác hẳn nhau, và **tự sửa rồi mới báo cáo** — nên
//! nếu người dùng chỉ nhìn thấy một thanh tiến trình bò ngang thì họ không có
//! cách nào biết nó đang làm gì với sách của mình cho tới lúc mọi thứ đã xong.
//! Cửa sổ nhật ký là chỗ trả lời câu ấy trong lúc còn kịp bấm dừng.
//!
//! Việc nặng chạy ở **luồng nền**: mô hình ngôn ngữ chấm hàng nghìn lượt cho một
//! cuốn sách, chạy trên luồng vẽ thì Windows treo biển "không phản hồi".

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chinhta::soat::ChamDiem;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use ungdung::nhat_ky::{Bao, Dong, Muc, Tin};
use ungdung::{bao_cao, cai_dat::CaiDat, xu_ly};

fn main() -> eframe::Result<()> {
    let tuy_chon = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 860.0])
            .with_min_inner_size([680.0, 540.0])
            .with_title("Sửa chính tả EPUB"),
        ..Default::default()
    };
    eframe::run_native(
        "VieSpellcheck",
        tuy_chon,
        Box::new(|cc| {
            dat_phong_chu(&cc.egui_ctx);
            Ok(Box::new(UngDung::moi()))
        }),
    )
}

pub const PHONG_UNG_VIEN: [&str; 5] = [
    r"C:\Windows\Fonts\segoeui.ttf",
    r"C:\Windows\Fonts\tahoma.ttf",
    r"C:\Windows\Fonts\arial.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
];

/// Nạp một phông chữ có tiếng Việt từ hệ điều hành.
///
/// Phông đi kèm egui **không có** khoảng Latin mở rộng bổ sung (U+1EA0–U+1EF9),
/// tức là không có `ạ ả ấ ầ ệ ộ ợ ự` — hơn nửa số chữ có dấu của tiếng Việt.
/// Thiếu nó thì giao diện đầy ô vuông trống, mà lỗi này chỉ lộ ra khi chạy thật
/// chứ không bài kiểm logic nào bắt được (`tests/phong_chu.rs` canh chỗ này).
///
/// Lấy phông của hệ điều hành thay vì nhúng một file vào chương trình: nhúng
/// thì binary phồng thêm vài megabyte và phải trông chừng giấy phép phông.
fn dat_phong_chu(ctx: &egui::Context) {
    let Some(byte) = PHONG_UNG_VIEN.iter().find_map(|p| std::fs::read(p).ok()) else {
        return;
    };
    let mut phong = egui::FontDefinitions::default();
    phong
        .font_data
        .insert("he_dieu_hanh".into(), std::sync::Arc::new(egui::FontData::from_owned(byte)));
    // Chèn lên **đầu** danh sách chứ không thêm vào cuối: egui lấy chữ từ phông
    // đầu tiên có glyph, mà phông mặc định có sẵn `a à á` nên nếu nó đứng trước
    // thì một câu tiếng Việt bị vẽ bằng hai phông trộn lẫn, cao thấp so le.
    for ho in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        phong.families.entry(ho).or_default().insert(0, "he_dieu_hanh".into());
    }
    ctx.set_fonts(phong);
}

/// Tin từ luồng nền. Bọc thêm hai trạng thái kết thúc quanh [`Tin`] của lõi.
enum TinUi {
    Tien(Tin),
    Xong(Box<xu_ly::KetQuaSach>, PathBuf, Option<PathBuf>),
    Loi(String),
}

/// Trần số dòng nhật ký giữ trong bộ nhớ.
///
/// Một bộ truyện 2.500 chương sinh ra vài vạn dòng. Giữ hết thì cửa sổ ăn dần
/// bộ nhớ rồi ì ra đúng lúc đang chạy việc nặng — mà phần người ta thật sự đọc
/// bao giờ cũng là phần cuối.
const TRAN_NHAT_KY: usize = 20_000;

struct UngDung {
    cai_dat: CaiDat,
    sach: Option<PathBuf>,
    /// Chỗ lưu do người dùng chọn. `None` thì dùng tên gợi ý bên cạnh bản gốc.
    ///
    /// Giữ riêng chứ không nhét vào [`CaiDat`]: đây là lựa chọn cho **một cuốn
    /// sách**, không phải thói quen. Lưu xuống đĩa rồi mở lại thì cuốn sau bị
    /// ghi đè lên kết quả của cuốn trước.
    noi_luu: Option<PathBuf>,
    ty_le: f32,
    mo_ta: String,
    dang_chay: bool,
    nhan: Option<mpsc::Receiver<TinUi>>,
    nhat_ky: Vec<Dong>,
    /// Số dòng đã bị cắt khỏi đầu danh sách vì chạm trần.
    da_cat: usize,
    hien_chi_tiet: bool,
    ket_qua: Option<Box<xu_ly::KetQuaSach>>,
    file_ra: Option<PathBuf>,
    file_bao_cao: Option<PathBuf>,
    loi: Option<String>,
    hien_cai_dat: bool,
    mo_hinh_co_san: Vec<PathBuf>,
}

impl UngDung {
    fn moi() -> UngDung {
        let cai_dat = CaiDat::nap();
        UngDung {
            mo_hinh_co_san: tim_mo_hinh(cai_dat.mo_hinh.as_deref()),
            cai_dat,
            sach: None,
            noi_luu: None,
            ty_le: 0.0,
            mo_ta: String::new(),
            dang_chay: false,
            nhan: None,
            nhat_ky: Vec::new(),
            da_cat: 0,
            hien_chi_tiet: false,
            ket_qua: None,
            file_ra: None,
            file_bao_cao: None,
            loi: None,
            hien_cai_dat: false,
        }
    }

    /// Chỗ sẽ ghi kết quả: người dùng chọn, hoặc tên gợi ý cạnh bản gốc.
    fn dich(&self) -> Option<PathBuf> {
        let vao = self.sach.as_ref()?;
        Some(self.noi_luu.clone().unwrap_or_else(|| xu_ly::ten_ra(vao)))
    }

    fn bat_dau(&mut self) {
        let Some(vao) = self.sach.clone() else { return };
        let Some(ra) = self.dich() else { return };
        // Chặn cuối cùng trước khi ghi. Người dùng bấm "Lưu thành…" rồi chọn
        // đúng file đang mở là mất bản gốc — thứ duy nhất có để đối chiếu nếu
        // bộ sửa làm sai. Nút đã bị khoá ở giao diện, nhưng chặn cả ở đây vì
        // đây mới là chỗ ghi thật.
        if ra == vao {
            self.loi = Some("Chỗ lưu trùng file gốc. Bản gốc phải giữ nguyên.".into());
            return;
        }
        let tuy_chon = self.cai_dat.thanh_tuy_chon();
        let duong_mo_hinh = self.cai_dat.mo_hinh.clone();
        let viet_bao_cao = self.cai_dat.viet_bao_cao;
        let (gui, nhan) = mpsc::channel();
        self.nhan = Some(nhan);
        self.dang_chay = true;
        self.ty_le = 0.0;
        self.mo_ta = "bắt đầu…".into();
        self.nhat_ky.clear();
        self.da_cat = 0;
        self.ket_qua = None;
        self.loi = None;

        std::thread::spawn(move || {
            let mut day = |t: Tin| {
                let _ = gui.send(TinUi::Tien(t));
            };
            let mut bao = Bao::moi(&mut day);

            // Nạp mô hình **trong luồng này**. `MoHinh` giữ con trỏ vào ngữ cảnh
            // llama.cpp nên không gửi qua luồng được.
            let mo_hinh = match &duong_mo_hinh {
                None => {
                    bao.buoc("Không dùng mô hình ngôn ngữ — chỉ chạy các tầng luật");
                    None
                }
                Some(p) => {
                    let ten = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let co = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                    bao.buoc(format!("Nạp mô hình {ten} ({:.1} GB)", co as f64 / 1e9));
                    match mohinh::card_dung_duoc() {
                        Some(c) => bao.chi_tiet(format!(
                            "card: {} — {:.1} GB VRAM, còn trống {:.1} GB",
                            c.description,
                            c.memory_total as f64 / 1e9,
                            c.memory_free as f64 / 1e9
                        )),
                        None => bao.canh_bao(format!(
                            "llama.cpp chỉ thấy: {}",
                            mohinh::liet_ke_thiet_bi()
                        )),
                    }

                    // Tiến độ nạp đi qua một kênh riêng: `Bao` đang bị mượn
                    // `&mut` nên callback không dùng lại nó được.
                    let gui2 = gui.clone();
                    let kq = mohinh::MoHinh::nap_co_tien_do(p, move |t| {
                        let _ = gui2.send(TinUi::Tien(Tin::TienDo {
                            ty_le: t * 0.12,
                            mo_ta: format!("nạp mô hình… {:.0}%", t * 100.0),
                        }));
                    });
                    match kq {
                        Ok(m) => {
                            bao.buoc(format!("Mô hình sẵn sàng: {}", m.mo_ta()));
                            Some(m)
                        }
                        Err(e) => {
                            // Chọn mô hình rồi thì mô hình **phải chạy được**.
                            // Nạp hỏng mà âm thầm chạy tiếp bằng luật là trả về
                            // một cuốn sách khác với cuốn người dùng đã đặt.
                            let _ = gui.send(TinUi::Loi(format!("{e:#}")));
                            return;
                        }
                    }
                }
            };

            let kq = xu_ly::xu_ly(
                &vao,
                &ra,
                tuy_chon,
                mo_hinh.as_ref().map(|m| m as &dyn ChamDiem),
                &mut bao,
            );
            match kq {
                Ok(kq) => {
                    let bc = if viet_bao_cao {
                        let p = ra.with_extension("bao-cao.html");
                        match std::fs::write(&p, bao_cao::html(&kq)) {
                            Ok(_) => {
                                bao.buoc(format!("Ghi báo cáo {}", p.display()));
                                Some(p)
                            }
                            Err(e) => {
                                bao.canh_bao(format!("không ghi được báo cáo: {e}"));
                                None
                            }
                        }
                    } else {
                        None
                    };
                    if let Some(m) = &mo_hinh {
                        bao.chi_tiet(format!("mô hình chạy {} lượt chấm", m.so_luot.get()));
                    }
                    drop(bao);
                    let _ = gui.send(TinUi::Xong(Box::new(kq), ra, bc));
                }
                Err(e) => {
                    drop(bao);
                    let _ = gui.send(TinUi::Loi(format!("{e:#}")));
                }
            }
        });
    }

    fn doc_tin(&mut self, ctx: &egui::Context) {
        let Some(nhan) = &self.nhan else { return };
        let mut ket_thuc = false;
        // Rút cạn hàng đợi mỗi khung hình. Chạy việc nặng thì nhật ký đổ về
        // nhanh hơn tốc độ vẽ, nên lấy từng cái một là tụt lại mãi mãi.
        while let Ok(t) = nhan.try_recv() {
            match t {
                TinUi::Tien(Tin::TienDo { ty_le, mo_ta }) => {
                    self.ty_le = ty_le;
                    self.mo_ta = mo_ta;
                }
                TinUi::Tien(Tin::Ghi(d)) => self.nhat_ky.push(d),
                TinUi::Xong(kq, ra, bc) => {
                    self.ket_qua = Some(kq);
                    self.file_ra = Some(ra);
                    self.file_bao_cao = bc;
                    ket_thuc = true;
                }
                TinUi::Loi(e) => {
                    self.loi = Some(e);
                    ket_thuc = true;
                }
            }
        }
        if self.nhat_ky.len() > TRAN_NHAT_KY {
            let bo = self.nhat_ky.len() - TRAN_NHAT_KY;
            self.nhat_ky.drain(..bo);
            self.da_cat += bo;
        }
        if ket_thuc {
            self.dang_chay = false;
            self.nhan = None;
        } else {
            // Luồng nền không đánh thức được luồng vẽ, nên phải tự hẹn vẽ lại;
            // không thì thanh tiến trình chỉ nhúc nhích khi người dùng rê chuột.
            ctx.request_repaint_after(std::time::Duration::from_millis(70));
        }
    }
}

impl eframe::App for UngDung {
    fn update(&mut self, ctx: &egui::Context, _khung: &mut eframe::Frame) {
        self.doc_tin(ctx);
        self.nhan_tha_file(ctx);

        egui::TopBottomPanel::top("tren").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("Sửa chính tả EPUB tiếng Việt");
            ui.label(
                egui::RichText::new(
                    "Bản gốc không bị đụng tới — sách đã sửa ghi ra một file mới bên cạnh.",
                )
                .weak(),
            );
            ui.add_space(10.0);
            self.phan_chon_sach(ui);
            ui.add_space(6.0);
            self.phan_cai_dat(ui);
            ui.add_space(10.0);
            self.phan_chay(ui);
            ui.add_space(8.0);
        });

        if self.ket_qua.is_some() || self.loi.is_some() {
            egui::TopBottomPanel::bottom("duoi").resizable(true).show(ctx, |ui| {
                ui.add_space(6.0);
                self.phan_ket_qua(ui);
                ui.add_space(6.0);
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| self.phan_nhat_ky(ui));
    }
}

impl UngDung {
    /// Kéo file EPUB thả thẳng vào cửa sổ.
    fn nhan_tha_file(&mut self, ctx: &egui::Context) {
        if self.dang_chay {
            return;
        }
        ctx.input(|i| {
            if let Some(p) = i.raw.dropped_files.first().and_then(|f| f.path.clone()) {
                if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("epub")) {
                    self.sach = Some(p);
                    self.noi_luu = None;
                    self.ket_qua = None;
                    self.loi = None;
                }
            }
        });
    }

    fn phan_chon_sach(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.dang_chay, egui::Button::new("Chọn file EPUB…")).clicked() {
                    if let Some(p) =
                        rfd::FileDialog::new().add_filter("Sách EPUB", &["epub"]).pick_file()
                    {
                        self.sach = Some(p);
                        // Đổi sách thì bỏ chỗ lưu cũ — không thì cuốn mới ghi
                        // đè lên kết quả của cuốn trước.
                        self.noi_luu = None;
                        self.ket_qua = None;
                        self.loi = None;
                    }
                }
                match &self.sach {
                    Some(p) => {
                        ui.label(
                            egui::RichText::new(p.file_name().unwrap_or_default().to_string_lossy())
                                .strong(),
                        );
                    }
                    None => {
                        ui.label(egui::RichText::new("hoặc kéo file thả vào đây").weak());
                    }
                }
            });

            if self.sach.is_none() {
                return;
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.dang_chay, egui::Button::new("Lưu thành…")).clicked() {
                    let goi_y = self.dich().unwrap_or_default();
                    let hop = rfd::FileDialog::new()
                        .add_filter("Sách EPUB", &["epub"])
                        .set_file_name(goi_y.file_name().unwrap_or_default().to_string_lossy())
                        .set_directory(goi_y.parent().unwrap_or(Path::new(".")));
                    if let Some(p) = hop.save_file() {
                        if Some(&p) == self.sach.as_ref() {
                            self.loi =
                                Some("Chỗ lưu trùng file gốc. Bản gốc phải giữ nguyên.".into());
                        } else {
                            self.noi_luu = Some(p);
                            self.loi = None;
                        }
                    }
                }
                if let Some(d) = self.dich() {
                    ui.label(egui::RichText::new(d.to_string_lossy()).weak());
                }
                if self.noi_luu.is_some()
                    && ui.add_enabled(!self.dang_chay, egui::Button::new("mặc định")).clicked()
                {
                    self.noi_luu = None;
                }
            });
        });
    }

    fn phan_cai_dat(&mut self, ui: &mut egui::Ui) {
        let bam = ui
            .horizontal(|ui| {
                let n = ui.selectable_label(self.hien_cai_dat, "Cài đặt").clicked();
                ui.label(
                    egui::RichText::new(match &self.cai_dat.mo_hinh {
                        Some(p) => format!(
                            "· mô hình: {}",
                            p.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        None => "· không dùng mô hình ngôn ngữ".into(),
                    })
                    .weak(),
                );
                n
            })
            .inner;
        if bam {
            self.hien_cai_dat = !self.hien_cai_dat;
        }
        if !self.hien_cai_dat {
            return;
        }

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                let cd = &mut self.cai_dat;
                ui.label(egui::RichText::new("SỬA GÌ").weak().small());
                ui.checkbox(&mut cd.unicode, "Dựng lại Unicode, bỏ ký tự vô hình");
                ui.checkbox(&mut cd.khoang_trang, "Khoảng trắng thừa");
                ui.checkbox(&mut cd.dau_cau, "Khoảng trắng quanh dấu câu");
                ui.checkbox(&mut cd.gom_dau_cham, "Gộp bốn chấm trở lên về ba");
                ui.checkbox(&mut cd.dung_ky_tu_ba_cham, "Đổi ... thành ký tự …");
                ui.checkbox(&mut cd.nhat_quan_dau_thanh, "Thống nhất kiểu đặt dấu (hòa / hoà)");
                ui.checkbox(&mut cd.am_tiet_sai, "Tiếng sai cấu tạo");
                ui.checkbox(&mut cd.de_nham, "Cặp dễ nhầm (xử dụng, câu truyện…)");
                ui.checkbox(&mut cd.chu_khong_dau, "Cả chữ không dấu");
                if cd.chu_khong_dau {
                    ui.label(
                        egui::RichText::new(
                            "  (!) Chữ không dấu không phân được với từ tiếng Anh — sách dịch \
                             nhiều tên riêng nước ngoài thì nên tắt.",
                        )
                        .weak()
                        .small(),
                    );
                }
                ui.checkbox(&mut cd.viet_bao_cao, "Ghi báo cáo HTML cạnh sách");

                ui.add_space(10.0);
                ui.label(egui::RichText::new("MÔ HÌNH NGÔN NGỮ").weak().small());
                ui.label(
                    egui::RichText::new(
                        "Không bắt buộc. Có mô hình thì những chỗ phải hiểu câu mới phân được \
                         — chia sẻ / chia xẻ, dành / giành — mới được sửa; không có thì chúng \
                         nằm nguyên trong mục “chỗ ngờ” của báo cáo. Chạy trên card đồ hoạ; \
                         không có card thì báo lỗi chứ không chạy bằng CPU.",
                    )
                    .weak()
                    .small(),
                );
                self.chon_mo_hinh(ui);
                if self.cai_dat.mo_hinh.is_some() {
                    ui.add(
                        egui::Slider::new(&mut self.cai_dat.nguong_mo_hinh, 0.0..=0.6)
                            .text("ngưỡng tin mô hình"),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Mô hình phải chấm cách sửa hơn bản gốc quá mức này thì mới đổi. \
                             Kéo lên là dè dặt hơn — sửa ít, sai ít.",
                        )
                        .weak()
                        .small(),
                    );
                }
            });
        });
    }

    fn chon_mo_hinh(&mut self, ui: &mut egui::Ui) {
        let ten = |p: &Path| p.file_name().unwrap_or_default().to_string_lossy().to_string();
        let hien_tai = match &self.cai_dat.mo_hinh {
            Some(p) => ten(p),
            None => "(không dùng)".to_string(),
        };
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("mo_hinh").selected_text(hien_tai).show_ui(ui, |ui| {
                let mut chon = self.cai_dat.mo_hinh.clone();
                ui.selectable_value(&mut chon, None, "(không dùng)");
                for p in &self.mo_hinh_co_san {
                    let nhan = match std::fs::metadata(p).map(|m| m.len()) {
                        Ok(n) => format!("{}  ({:.1} GB)", ten(p), n as f64 / 1e9),
                        Err(_) => ten(p),
                    };
                    ui.selectable_value(&mut chon, Some(p.clone()), nhan);
                }
                self.cai_dat.mo_hinh = chon;
            });
            if ui.button("Chọn file .gguf khác…").clicked() {
                if let Some(p) =
                    rfd::FileDialog::new().add_filter("Mô hình GGUF", &["gguf"]).pick_file()
                {
                    if mohinh::la_gguf(&p) {
                        if !self.mo_hinh_co_san.contains(&p) {
                            self.mo_hinh_co_san.push(p.clone());
                        }
                        self.cai_dat.mo_hinh = Some(p);
                    } else {
                        self.loi = Some("File chọn không phải định dạng GGUF.".into());
                    }
                }
            }
        });
    }

    fn phan_chay(&mut self, ui: &mut egui::Ui) {
        if self.dang_chay {
            ui.add(egui::ProgressBar::new(self.ty_le).show_percentage().animate(true));
            ui.label(egui::RichText::new(&self.mo_ta).weak());
            return;
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.sach.is_some(),
                    egui::Button::new(egui::RichText::new("  Kiểm và sửa  ").strong()),
                )
                .clicked()
            {
                self.cai_dat.luu();
                self.bat_dau();
            }
            if let Some(d) = self.dich() {
                ui.label(
                    egui::RichText::new(format!(
                        "→ {}",
                        d.file_name().unwrap_or_default().to_string_lossy()
                    ))
                    .weak(),
                );
            }
        });
    }

    fn phan_nhat_ky(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Nhật ký").strong());
            ui.checkbox(&mut self.hien_chi_tiet, "hiện từng chỗ sửa");
            if ui.button("Chép").clicked() {
                let t: String = self
                    .nhat_ky
                    .iter()
                    .map(|d| format!("[{:7.2}s] {}{}\n", d.giay, d.muc.dau(), d.chu))
                    .collect();
                ui.ctx().copy_text(t);
            }
            if self.da_cat > 0 {
                ui.label(
                    egui::RichText::new(format!("(đã cắt {} dòng đầu)", self.da_cat))
                        .weak()
                        .small(),
                );
            }
        });
        ui.separator();

        // Lấy màu "chữ mờ" ra **trước** vòng vẽ: đọc `ui.visuals()` bên trong
        // closure là mượn `ui` bất biến, mà thân vòng lại cần mượn `ui` khả biến.
        let mo = ui.visuals().weak_text_color();
        let mau = move |m: Muc| match m {
            Muc::Buoc => egui::Color32::from_rgb(120, 170, 255),
            Muc::CanhBao => egui::Color32::from_rgb(230, 175, 80),
            Muc::ChiTiet => mo,
        };
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            // Dính đáy: người ta mở nhật ký ra để nhìn cái **vừa xảy ra**. Nhưng
            // chỉ dính khi đang chạy — chạy xong mà vẫn kéo về đáy thì không đọc
            // lại được đoạn giữa.
            .stick_to_bottom(self.dang_chay)
            .show(ui, |ui| {
                for d in self.nhat_ky.iter() {
                    if d.muc == Muc::ChiTiet && !self.hien_chi_tiet {
                        continue;
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(
                            egui::RichText::new(format!("{:7.2}s", d.giay))
                                .monospace()
                                .small()
                                .weak(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}{}", d.muc.dau(), d.chu))
                                .monospace()
                                .small()
                                .color(mau(d.muc)),
                        );
                    });
                }
                if self.nhat_ky.is_empty() {
                    ui.label(
                        egui::RichText::new("Chọn một cuốn EPUB rồi bấm “Kiểm và sửa”.")
                            .weak()
                            .small(),
                    );
                }
            });
    }

    fn phan_ket_qua(&mut self, ui: &mut egui::Ui) {
        if let Some(e) = &self.loi {
            ui.colored_label(egui::Color32::from_rgb(220, 90, 80), format!("Lỗi: {e}"));
            return;
        }
        let Some(kq) = &self.ket_qua else { return };

        ui.label(egui::RichText::new(bao_cao::mot_dong(kq)).strong());
        let ngo_vuc = bao_cao::so_ngo_vuc(kq);
        if ngo_vuc > 0 {
            ui.label(
                egui::RichText::new(format!(
                    "Trong đó {ngo_vuc} chỗ do mô hình quyết — soi kỹ mục này trong báo cáo.",
                ))
                .weak()
                .small(),
            );
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if let Some(p) = self.file_bao_cao.clone() {
                if ui.button("Mở báo cáo").clicked() {
                    mo_file(&p);
                }
            }
            if let Some(p) = self.file_ra.clone() {
                if ui.button("Mở thư mục chứa sách").clicked() {
                    if let Some(d) = p.parent() {
                        mo_file(d);
                    }
                }
            }
            for (loai, n) in kq.dem_theo_loai() {
                ui.label(egui::RichText::new(format!("{loai}: {n}")).weak().small());
            }
        });
    }
}

/// Tìm các file `.gguf` ở những chỗ quen thuộc, để khỏi phải mở hộp thoại.
///
/// Quét sâu **một cấp** dưới mỗi thư mục gốc: mô hình tải về hay nằm trong một
/// thư mục con mang tên chính nó. Quét sâu hơn thì đụng cache của HuggingFace,
/// nơi có hàng chục file trùng tên nằm trong các thư mục băm khó đọc.
fn tim_mo_hinh(dang_dung: Option<&Path>) -> Vec<PathBuf> {
    let mut ra: Vec<PathBuf> = Vec::new();
    let goc: Vec<PathBuf> =
        [Some(PathBuf::from(r"C:\Dev\models")), dirs_mo_hinh()].into_iter().flatten().collect();

    for g in goc {
        let Ok(muc) = std::fs::read_dir(&g) else { continue };
        for m in muc.flatten() {
            let p = m.path();
            if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gguf")) {
                ra.push(p);
            } else if p.is_dir() {
                if let Ok(con) = std::fs::read_dir(&p) {
                    for c in con.flatten() {
                        let q = c.path();
                        if q.extension().is_some_and(|e| e.eq_ignore_ascii_case("gguf")) {
                            ra.push(q);
                        }
                    }
                }
            }
        }
    }
    if let Some(p) = dang_dung {
        if !ra.iter().any(|x| x == p) {
            ra.push(p.to_path_buf());
        }
    }
    ra.sort();
    ra.dedup();
    ra
}

fn dirs_mo_hinh() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("models"))
}

/// Mở file hoặc thư mục bằng chương trình mặc định của hệ điều hành.
fn mo_file(p: &Path) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", &p.to_string_lossy()]).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(p).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(p).spawn();
}
