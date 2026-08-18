//! Chấm điểm câu bằng một mô hình ngôn ngữ GGUF chạy trên máy.
//!
//! # Vì sao **chấm điểm** chứ không **hỏi** mô hình
//!
//! Cách hiển nhiên là đưa câu cho mô hình rồi bảo "sửa lỗi chính tả giúp". Cách
//! ấy sai ở đây, và sai theo kiểu không vá được:
//!
//! - Mô hình **viết lại văn của tác giả**. Bảo nó sửa chính tả thì nó tiện tay
//!   đổi luôn từ ngữ, gộp câu, bỏ chữ nó thấy thừa. Trong một cuốn tiểu thuyết
//!   thì đó không phải sửa lỗi, đó là hỏng sách.
//! - Mô hình nhỏ **bịa**. Gặp tên riêng lạ nó đổi thành tên nó biết.
//! - Kết quả **không lặp lại được**, nên không kiểm thử được.
//!
//! Ở đây mô hình không được sinh ra chữ nào. Nó chỉ làm đúng một việc: cho một
//! câu, trả về log-xác suất trung bình mỗi token. Các cách sửa thì do tầng luật
//! và tầng cấu tạo âm tiết sinh ra — chúng bảo đảm mọi ứng viên đều là tiếng
//! Việt viết đúng — còn mô hình chỉ **xếp hạng** chúng. Chuỗi mô hình chưa từng
//! thấy thì không cách nào lọt vào sách được, vì nó không tồn tại trong danh
//! sách ứng viên.
//!
//! # Hai cách hỏi cùng một mô hình
//!
//! [`MoHinh::cham`] chấm **cả câu**: mỗi ứng viên thay vào rồi chấm lại từ đầu.
//! [`MoHinh::cham_cho_trong`] khoét chỗ chữ sai thành **chỗ trống** rồi chỉ chấm
//! phần điền vào cùng mấy chữ theo sau, còn phần ngữ cảnh đứng trước — hai câu
//! trước và đầu câu hiện tại — chỉ để mô hình đọc chứ không tính điểm.
//!
//! Đo trên tập 4 Harry Potter, cùng ngưỡng 0,03, cả hai bắt gần bằng nhau (126
//! so với 122 lỗi chữ nghĩa, 0 chỗ ngờ để lại) và quyết khác nhau ở 8 trong 86
//! chỗ mô hình được hỏi. Cái khác nhau **không** nằm ở số lượng mà ở chỗ điểm số
//! có dùng làm độ tin cậy được hay không:
//!
//! | chỗ sửa | đúng phải | cả câu | chỗ trống |
//! |---|---|---|---|
//! | `thuớc` → `thước` | sửa | +0,097 | +0,115 |
//! | `bôj` → `bộ` | sửa | +0,105 | +0,142 |
//! | `zợi` → `sợi` | **giữ** (giọng nhân vật) | **+0,504** | −0,024 |
//! | `ghứ` → `chữ` | **giữ** | **+0,715** | +0,210 |
//! | `shứ` → `chứ` | **giữ** | **+0,317** | +0,190 |
//!
//! Chấm cả câu thì mấy ca **sai** lại được điểm cao gấp mấy lần mấy ca đúng, nên
//! không ngưỡng nào tách được chúng: nâng ngưỡng lên là mất phép sửa thật trước
//! khi chặn được phép sửa nhầm. Lối điền chỗ trống đảo lại đúng thứ tự ấy, và đó
//! là lý do nó chịu được ngưỡng rộng gấp hai mươi lần (xem `examples/so_loi_cham.rs`).
//!
//! Giá phải trả là **thời gian**: 69 giây lên 106…115 giây (đo nhiều lượt) cho
//! cùng 1.427 lượt chấm, vì phần ngữ cảnh đứng trước dài hơn hẳn một câu. Phần ấy
//! giống nhau ở mọi ứng viên của cùng một chỗ sửa nên bộ nhớ đệm KV giữ lại được
//! — 86 lượt nạp thay vì 1.427 — nhưng vẫn phải nạp một lần cho mỗi chỗ.
//!
//! Điều **không** đổi là mô hình vẫn không viết ra chữ nào: nó chấm những chữ
//! ta đưa cho nó điền vào chỗ trống, chứ không được tự chọn chữ ngoài danh sách.
//!
//! # Chọn mô hình
//!
//! Cỡ nào cũng chạy được, nhưng cỡ có ảnh hưởng thật, và ảnh hưởng **không đều**
//! giữa hai việc mà tầng này làm:
//!
//! - Chọn giữa `thuơng` và `thương` thì mô hình nào cũng làm được — một bên là
//!   chuỗi ký tự mô hình chưa từng thấy, chênh lệch điểm rất lớn.
//! - Chọn giữa `để dành` và `để giành`, `chia sẻ` và `chia xẻ` thì phải **hiểu
//!   câu**. Cả hai đều là tiếng Việt trôi chảy, chênh lệch điểm nhỏ, và mô hình
//!   nhỏ hay chấm sai hướng — mà ngưỡng an toàn ở [`TuyChon::nguong_mo_hinh`]
//!   thì biến "chấm sai hướng" thành "không sửa gì", tức là mất luôn tác dụng.
//!
//! Nên mô hình lớn đáng giá đúng ở lớp lỗi thứ hai. Bản dựng mặc định bật CUDA
//! và đẩy toàn bộ mô hình sang GPU (xem [`SO_LOP_GPU`]), nên một mô hình 9 tỷ
//! tham số lượng tử Q4 chạy được thoải mái trên card 16 GB.
//!
//! [`TuyChon::nguong_mo_hinh`]: chinhta::soat::TuyChon::nguong_mo_hinh

use anyhow::{Context, Result};
use chinhta::soat::ChamDiem;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::token::LlamaToken;
use std::cell::RefCell;
use std::num::NonZeroU32;
use std::path::Path;

/// Số token tối đa cho một lượt chấm. Chuỗi dài hơn bị cắt bớt đầu.
///
/// Chấm cả câu thì 512 đã quá thừa, nhưng lối điền chỗ trống đưa vào hai câu
/// trước — đo trên sách thật thì cửa sổ ấy dài tới 800 ký tự, mà tiếng Việt qua
/// bộ tách token của Qwen ra khoảng ba ký tự một token, tức là ngót 300 token.
/// Để 512 thì gặp đoạn văn dài là cắt mất đúng phần ngữ cảnh vừa thêm vào. Chỗ
/// này rẻ: 1024 token bộ nhớ đệm KV của mô hình 9 tỷ tham số chỉ tốn vài chục MB.
const NGU_CANH: u32 = 1024;

/// Số lớp đẩy sang GPU. Đặt cao hơn số lớp của mọi mô hình để nó **đẩy hết**;
/// llama.cpp tự cắt xuống đúng số lớp thật có.
///
/// Vì sao đẩy hết chứ không chia đôi: chỉ cần một lớp nằm lại CPU là mỗi lượt
/// chấm phải chuyển trạng thái qua lại qua khe PCIe, và với hàng nghìn lượt câu
/// ngắn thì phần chuyển ấy át hẳn phần tính.
const SO_LOP_GPU: u32 = 999;

/// Ngưỡng VRAM tối thiểu để nhận là card dùng được.
///
/// Dưới mức này thì mô hình không nằm trọn trong VRAM, llama.cpp phải để một
/// phần lớp lại CPU, và ta rơi đúng vào cái đã bị cấm — chỉ có điều rơi vào một
/// cách âm thầm. Thà báo lỗi.
const VRAM_TOI_THIEU: usize = 4 * 1024 * 1024 * 1024;

/// Tiền tố tên ba DLL runtime của CUDA mà bản dựng này cần.
const DLL_CAN: [&str; 3] = ["cublas64_", "cublasLt64_", "cudart64_"];

/// Thư mục chứa file thực thi đang chạy.
fn thu_muc_exe() -> Option<std::path::PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.to_path_buf())
}

/// Đã có đủ DLL runtime của CUDA cạnh file thực thi chưa.
///
/// **Phải gọi trước mọi thứ khác trong crate này.** Bản dựng Windows cho cuBLAS
/// *nạp trễ* (xem `giaodien/build.rs`), nên ứng dụng mở được trên máy chưa cài
/// CUDA — nhưng đổi lại, hàm llama.cpp đầu tiên được gọi lúc thiếu DLL sẽ giết
/// tiến trình ngay tại chỗ, không có lỗi nào bắt được. Cái van duy nhất là đừng
/// gọi.
///
/// Chỉ tìm cạnh exe chứ không tra PATH, và đó là chủ ý: CUDA 13 dời DLL runtime
/// sang thư mục `bin/x64` nên máy vừa cài Toolkit xong vẫn không có chúng trong
/// PATH.
/// Cạnh exe cũng đúng chỗ mà `giaodien::tai_cuda` tải về.
pub fn du_dll() -> bool {
    if !cfg!(all(windows, feature = "cuda")) {
        return true;
    }
    let Some(d) = thu_muc_exe() else { return false };
    let Ok(muc) = std::fs::read_dir(&d) else { return false };
    let ten: Vec<String> =
        muc.filter_map(|x| Some(x.ok()?.file_name().to_string_lossy().to_string())).collect();
    DLL_CAN.iter().all(|t| ten.iter().any(|n| n.starts_with(t) && n.ends_with(".dll")))
}

/// Card đồ hoạ dùng được, nếu có.
///
/// Chỉ nhận GPU rời (`Gpu`). GPU tích hợp bị loại: nó dùng chung RAM với hệ
/// thống nên không nhanh hơn CPU là bao, mà lại làm ta tưởng đang chạy trên card.
pub fn card_dung_duoc() -> Option<llama_cpp_2::LlamaBackendDevice> {
    // Thiếu DLL mà gọi xuống llama.cpp là chết tiến trình — xem [`du_dll`].
    if !du_dll() {
        return None;
    }
    llama_cpp_2::list_llama_ggml_backend_devices()
        .into_iter()
        .filter(|d| d.device_type == llama_cpp_2::LlamaBackendDeviceType::Gpu)
        .max_by_key(|d| d.memory_total)
}

/// Mô tả mọi thiết bị llama.cpp nhìn thấy — để in ra khi báo lỗi thiếu card.
pub fn liet_ke_thiet_bi() -> String {
    if !du_dll() {
        return "(chưa có DLL runtime của CUDA)".into();
    }
    let d = llama_cpp_2::list_llama_ggml_backend_devices();
    if d.is_empty() {
        return "(không thấy thiết bị nào)".into();
    }
    d.iter()
        .map(|x| {
            format!(
                "{} [{}] {:?} {:.1} GB",
                x.description,
                x.backend,
                x.device_type,
                x.memory_total as f64 / 1e9
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// llama.cpp chỉ cho khởi động nền **một lần cho cả tiến trình** — gọi
/// `LlamaBackend::init()` lần thứ hai là lỗi. Người dùng đổi mô hình giữa chừng
/// là chuyện thường, nên giữ nền ở đây thay vì trong từng [`MoHinh`].
static NEN: std::sync::OnceLock<Result<LlamaBackend, String>> = std::sync::OnceLock::new();

fn nen() -> Result<&'static LlamaBackend> {
    match NEN.get_or_init(|| LlamaBackend::init().map_err(|e| e.to_string())) {
        Ok(n) => Ok(n),
        Err(e) => anyhow::bail!("không khởi động được llama.cpp: {e}"),
    }
}

pub struct MoHinh {
    /// **Thứ tự khai báo trường ở đây là thứ tự thả.** `ngu_canh` mượn
    /// `mo_hinh`, nên nó phải đứng trước để bị thả trước; đảo lại là dùng bộ
    /// nhớ đã giải phóng lúc thoát ứng dụng.
    ngu_canh: RefCell<Option<llama_cpp_2::context::LlamaContext<'static>>>,
    /// Trong `Box` nên địa chỉ cố định — đó là điều kiện để mượn `'static` ở
    /// dưới là đúng.
    mo_hinh: Box<LlamaModel>,
    /// Token của phần **đứng trước chỗ trống** ở lượt chấm gần nhất, và nó đang
    /// còn nằm trong bộ nhớ đệm KV.
    ///
    /// Các ứng viên của cùng một chỗ sửa dùng chung y nguyên phần đứng trước, mà
    /// phần ấy là phần dài nhất — hai câu ngữ cảnh cộng đầu câu hiện tại. Giữ lại
    /// thì mỗi ứng viên chỉ phải chạy mấy token của chính nó cộng phần đuôi.
    ///
    /// Đáng làm vì số ứng viên mỗi chỗ **nhiều hơn tưởng**: đo trên tập 4 Harry
    /// Potter là 1.427 lượt chấm cho 86 chỗ sửa, tức 16,6 lượt một chỗ. Không giữ
    /// lại thì phần đứng trước bị nạp 1.427 lần thay vì 86 lần.
    ///
    /// Rỗng nghĩa là bộ đệm không còn tin được — [`MoHinh::cham`] xoá sạch bộ đệm
    /// nên nó phải xoá luôn dấu vết ở đây.
    dem_tien_to: RefCell<Vec<LlamaToken>>,
    /// Đếm số lượt, chỉ để hiện trong báo cáo.
    pub so_luot: std::cell::Cell<u64>,
    pub duong_dan: std::path::PathBuf,
}

impl MoHinh {
    /// Nạp mô hình. **Không có card đồ hoạ rời thì trả lỗi, không chạy bằng CPU.**
    ///
    /// Lùi về CPU nghe thì tử tế nhưng ở đây là cái bẫy: mô hình 9 tỷ tham số
    /// chấm trên CPU chậm hơn hàng chục lần, và người dùng bấm "Kiểm và sửa"
    /// rồi ngồi nhìn thanh tiến trình hàng giờ mà không hiểu vì sao — chương
    /// trình có báo gì đâu, nó vẫn đang chạy. Báo lỗi ngay thì người ta biết
    /// đường xử lý: cài driver, hoặc bỏ mô hình đi mà chạy bằng luật.
    pub fn nap(duong_dan: &Path) -> Result<MoHinh> {
        Self::nap_co_tien_do(duong_dan, |_| {})
    }

    /// `tien_do` nhận tỷ lệ 0…1 trong lúc đọc file mô hình.
    ///
    /// Nạp 5 GB từ đĩa sang VRAM mất vài giây tới vài chục giây tuỳ ổ, và trong
    /// suốt quãng ấy chương trình không làm gì khác. Không báo ra thì đó là
    /// quãng đứng hình dài nhất của cả lượt chạy.
    pub fn nap_co_tien_do(
        duong_dan: &Path,
        mut tien_do: impl FnMut(f32) + 'static,
    ) -> Result<MoHinh> {
        // Chặn **trước** mọi lời gọi xuống llama.cpp, kể cả `nen()`.
        if !du_dll() {
            anyhow::bail!(
                "chưa có DLL runtime của CUDA cạnh file thực thi. Mở lại ứng dụng để tải về, hoặc bỏ mô hình để chạy bằng luật."
            );
        }
        let nen = nen()?;

        if !cfg!(feature = "cuda") {
            anyhow::bail!(
                "bản dựng này không có CUDA nên mô hình chỉ chạy được bằng CPU. \
                 Dựng lại với tính năng `cuda`, hoặc bỏ mô hình để chạy bằng luật."
            );
        }
        let card = card_dung_duoc().ok_or_else(|| {
            anyhow::anyhow!(
                "không thấy card đồ hoạ rời nào. llama.cpp chỉ nhìn thấy: {}. \
                 Kiểm lại driver NVIDIA, hoặc bỏ mô hình để chạy bằng luật.",
                liet_ke_thiet_bi()
            )
        })?;
        if card.memory_total < VRAM_TOI_THIEU {
            anyhow::bail!(
                "card {} chỉ có {:.1} GB VRAM, không đủ để giữ trọn mô hình — \
                 phần tràn sẽ chạy bằng CPU. Dùng mô hình nhỏ hơn, hoặc bỏ mô hình.",
                card.description,
                card.memory_total as f64 / 1e9
            );
        }

        let tham_so = LlamaModelParams::default()
            .with_n_gpu_layers(SO_LOP_GPU)
            // Trả `true` nghĩa là "chạy tiếp"; trả `false` là huỷ giữa chừng.
            .with_progress_callback(move |t| {
                tien_do(t);
                true
            });
        let mo_hinh = Box::new(
            LlamaModel::load_from_file(nen, duong_dan, &tham_so)
                .with_context(|| format!("không nạp được mô hình {}", duong_dan.display()))?,
        );

        // Số luồng chỉ ăn thua ở đường lùi về CPU, nhưng ở đó nó ăn thua rất
        // lớn: llama.cpp mặc định **4 luồng** bất kể máy có bao nhiêu nhân.
        let so_luong = std::thread::available_parallelism()
            .map(|n| (n.get() / 2).clamp(4, 32))
            .unwrap_or(4) as i32;
        let tham_so_nc = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(NGU_CANH))
            .with_n_batch(NGU_CANH)
            .with_n_threads(so_luong)
            .with_n_threads_batch(so_luong);

        // `LlamaContext` mượn `LlamaModel` mà cả hai phải nằm chung một struct.
        // Rust không diễn đạt được kiểu tự tham chiếu ấy, nên kéo dài đời mượn
        // bằng tay. Đúng vì hai điều kiện: `mo_hinh` nằm trong `Box` nên không
        // dời chỗ khi struct bị di chuyển, và `ngu_canh` khai trước nên bị thả
        // trước.
        let muon: &'static LlamaModel = unsafe { &*(&*mo_hinh as *const LlamaModel) };
        let nc = muon.new_context(nen, tham_so_nc).context("không tạo được ngữ cảnh")?;

        Ok(MoHinh {
            ngu_canh: RefCell::new(Some(nc)),
            mo_hinh,
            dem_tien_to: RefCell::new(Vec::new()),
            so_luot: std::cell::Cell::new(0),
            duong_dan: duong_dan.to_path_buf(),
        })
    }

    /// Mô tả ngắn để hiện trong báo cáo.
    pub fn mo_ta(&self) -> String {
        let ty = self.mo_hinh.n_params() as f64 / 1e9;
        let ten = self
            .duong_dan
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        format!("{ten} ({ty:.2} tỷ tham số)")
    }

    /// Log-xác suất của câu, **chia cho số ký tự**.
    ///
    /// Phải chia cho cái gì đó: lấy tổng thì chuỗi dài luôn thua chuỗi ngắn,
    /// tức là mọi phép xoá chữ đều được thưởng.
    ///
    /// Nhưng chia cho **số token** thì hỏng theo kiểu khó thấy: các ứng viên
    /// tách ra số token khác nhau, và thêm một token dễ đoán có thể **nâng**
    /// điểm trung bình lên. Đo được: mô hình chấm `Hắn phảii đi ngay.` cao hơn
    /// `Hắn phải đi ngay.` 0,167 — câu sai thắng câu đúng, vì `phảii` tách thành
    /// `phải` + `i` và cái đuôi `i` ấy không đắt bằng mức nó kéo mẫu số lên.
    ///
    /// Số ký tự thì không phụ thuộc vào cách tách token, mà các ứng viên ở đây
    /// chỉ chênh nhau một hai ký tự nên mẫu số gần như không đổi — đúng cái ta
    /// cần khi so hai câu gần giống hệt nhau.
    fn cham_that(&self, cau: &str) -> Result<f32> {
        // `AddBos::Always` đọc dễ nhầm: nó **không** ép thêm BOS, nó bật cờ
        // `add_special` của llama.cpp, tức là "thêm token đặc biệt theo đúng
        // khai báo của mô hình". Mô hình nào khai không dùng BOS — Qwen chẳng
        // hạn — thì không có gì được thêm. Đổi sang `Never` mới là sai, vì lúc
        // ấy mô hình nào *có* dùng BOS sẽ bị chấm trong một ngữ cảnh nó chưa
        // từng gặp lúc huấn luyện.
        let mut token = self
            .mo_hinh
            .str_to_token(cau, AddBos::Always)
            .context("không tách được token")?;
        if token.len() < 2 {
            return Ok(0.0);
        }
        if token.len() > NGU_CANH as usize {
            // Cắt phần đầu, giữ phần đuôi — chỗ sửa gần cuối cửa sổ hơn.
            token.drain(..token.len() - NGU_CANH as usize);
        }
        let n = token.len();

        let mut lo = LlamaBatch::new(n, 1);
        for (i, &t) in token.iter().enumerate() {
            // Cần logits ở **mọi** vị trí: xác suất của token i lấy từ phân bố
            // dự đoán ở vị trí i-1, nên phải giữ lại hết chứ không chỉ vị trí
            // cuối như lúc sinh chữ.
            lo.add(t, i as i32, &[0], true)?;
        }
        let mut muon = self.ngu_canh.borrow_mut();
        let nc = muon.as_mut().context("ngữ cảnh đã bị thả")?;
        // Xoá bộ nhớ đệm KV trước mỗi lượt: các lượt chấm **không** nối tiếp
        // nhau, mỗi câu là một chuỗi độc lập. Quên bước này thì câu sau được
        // chấm như thể nó viết tiếp câu trước, và điểm số thành vô nghĩa.
        nc.clear_kv_cache();
        // Xoá sạch bộ đệm thì tiền tố mà lối chấm chỗ trống đang giữ cũng mất
        // theo. Không ghi lại chuyện ấy thì lượt chấm chỗ trống sau tưởng tiền
        // tố còn nguyên, bỏ luôn bước nạp, và chấm ứng viên trong một ngữ cảnh
        // trống — hỏng lặng lẽ, không lỗi nào bật ra.
        self.dem_tien_to.borrow_mut().clear();
        nc.decode(&mut lo).context("mô hình không chạy được")?;

        let mut tong = 0.0f32;
        for i in 1..n {
            let logits = nc.get_logits_ith(i as i32 - 1);
            tong += log_xac_suat(logits, token[i].0 as usize);
        }
        self.so_luot.set(self.so_luot.get() + 1);
        let so_ky_tu = cau.chars().count().max(1) as f32;
        Ok(tong / so_ky_tu)
    }

    /// Log-xác suất của **phần điền vào chỗ trống cộng phần đuôi**, chia cho số
    /// ký tự của chính phần ấy. Phần `truoc` chỉ để mô hình đọc.
    ///
    /// Vì sao không tính điểm phần đứng trước: nó giống hệt nhau ở mọi ứng viên,
    /// nên nó chỉ cộng thêm một hằng số vào tử và một hằng số vào mẫu. Cộng vào
    /// mẫu là chỗ hại: hai câu ngữ cảnh dài gấp mấy lần chỗ sửa, và chênh lệch
    /// giữa `thương` với `thường` bị chia cho cả cụm ấy đến mức chìm dưới ngưỡng.
    ///
    /// Phần đuôi (`sau`) thì **phải** tính điểm dù nó cũng giống nhau ở mọi ứng
    /// viên. Đây là cách duy nhất để một mô hình chỉ đọc xuôi dùng được ngữ cảnh
    /// đứng sau: chữ điền vào không tự nói lên nó đúng, nhưng chữ **theo sau nó**
    /// thì trôi chảy hay gượng gạo tuỳ vào nó — `để dành` hay `để giành` phải nhìn
    /// tới hết câu mới phân được.
    fn cham_cho_trong_that(&self, truoc: &str, dien: &str, sau: &str) -> Result<f32> {
        // Khoảng trắng đứng ngay trước chỗ trống phải đi **theo phần được chấm**.
        // Bộ tách token kiểu BPE gắn khoảng trắng vào đầu chữ (` thương` là một
        // token, `thương` là token khác), nên cắt ngay sau khoảng trắng thì mọi
        // ứng viên bị chấm ở dạng "đứng đầu dòng" — dạng mà mô hình gần như không
        // gặp lúc huấn luyện, và điểm số vì thế mang thêm một lượng nhiễu chung.
        let (dau_chuoi, phan) = match truoc.strip_suffix(' ') {
            Some(x) => (x, format!(" {dien}{sau}")),
            None => (truoc, format!("{dien}{sau}")),
        };
        let mut tk_dau = self
            .mo_hinh
            .str_to_token(dau_chuoi, AddBos::Always)
            .context("không tách được token phần đứng trước")?;
        let tk_phan = self
            .mo_hinh
            .str_to_token(&phan, AddBos::Never)
            .context("không tách được token phần điền vào")?;
        // Không có gì để chấm, hoặc không có gì làm ngữ cảnh: trả 0 cho mọi ứng
        // viên, tức là không ai hơn ai và bản gốc thắng.
        if tk_phan.is_empty() || tk_dau.len() < 2 {
            return Ok(0.0);
        }
        if tk_dau.len() + tk_phan.len() > NGU_CANH as usize {
            // Cắt phần đầu ngữ cảnh, giữ phần gần chỗ trống. Phải chừa lại ít
            // nhất một token làm neo (xem dưới).
            let bo = (tk_dau.len() + tk_phan.len() - NGU_CANH as usize).min(tk_dau.len() - 1);
            tk_dau.drain(..bo);
        }

        // **Token cuối của phần đứng trước không nạp vào tiền tố** mà để lại làm
        // neo, chạy cùng ứng viên. Lý do: xác suất của token đầu tiên trong chỗ
        // trống lấy từ phân bố dự đoán ở vị trí ngay trước nó, mà `get_logits_ith`
        // chỉ đọc được logits của **lượt decode vừa rồi**. Nạp cả phần đứng trước
        // vào tiền tố thì logits ấy nằm ở lượt trước và mất khi chạy lượt ứng
        // viên — chỉ còn cách sao lại cả bảng logits (hơn 150 nghìn số thực cho
        // mỗi chỗ sửa) mới lấy lại được. Để lại một token thì rẻ hơn nhiều.
        let neo = tk_dau[tk_dau.len() - 1];
        let tien_to = &tk_dau[..tk_dau.len() - 1];
        let vt_neo = tien_to.len();

        let mut muon = self.ngu_canh.borrow_mut();
        let nc = muon.as_mut().context("ngữ cảnh đã bị thả")?;
        let mut dem = self.dem_tien_to.borrow_mut();
        if dem.as_slice() == tien_to {
            // Tiền tố còn nguyên trong bộ đệm — chỉ bỏ phần của ứng viên trước.
            // `clear_kv_cache_seq` trả `false` khi mô hình không cho xoá một
            // khúc giữa (mô hình hồi quy); lúc ấy nạp lại từ đầu cho chắc.
            let xoa_duoc = nc
                .clear_kv_cache_seq(Some(0), Some(vt_neo as u32), None)
                .unwrap_or(false);
            if !xoa_duoc {
                dem.clear();
            }
        }
        if dem.as_slice() != tien_to {
            nc.clear_kv_cache();
            let mut lo = LlamaBatch::new(tien_to.len().max(1), 1);
            for (i, &t) in tien_to.iter().enumerate() {
                // Chỉ cần logits ở token cuối, mà thật ra không cần cả nó —
                // nhưng llama.cpp muốn mỗi lượt decode có ít nhất một chỗ ra.
                lo.add(t, i as i32, &[0], i + 1 == tien_to.len())?;
            }
            nc.decode(&mut lo).context("mô hình không chạy được phần ngữ cảnh")?;
            *dem = tien_to.to_vec();
        }

        let mut lo = LlamaBatch::new(tk_phan.len() + 1, 1);
        lo.add(neo, vt_neo as i32, &[0], true)?;
        for (i, &t) in tk_phan.iter().enumerate() {
            // Token cuối không cần logits: sau nó không còn gì để chấm.
            lo.add(t, (vt_neo + 1 + i) as i32, &[0], i + 1 < tk_phan.len())?;
        }
        nc.decode(&mut lo).context("mô hình không chạy được phần điền vào")?;

        // Chỗ ra thứ `i` của lượt vừa rồi là phân bố dự đoán token đứng sau token
        // thứ `i` — tức là đúng `tk_phan[i]`, vì chỗ ra thứ 0 thuộc về neo.
        let mut tong = 0.0f32;
        for (i, &t) in tk_phan.iter().enumerate() {
            tong += log_xac_suat(nc.get_logits_ith(i as i32), t.0 as usize);
        }
        self.so_luot.set(self.so_luot.get() + 1);
        // Chia cho số ký tự, cùng lý do như [`MoHinh::cham_that`]: chia cho số
        // token thì thêm một token dễ đoán lại nâng điểm lên.
        let so_ky_tu = phan.chars().count().max(1) as f32;
        Ok(tong / so_ky_tu)
    }
}

/// log softmax tại một chỉ số, tính ổn định về mặt số học.
///
/// Trừ đi giá trị lớn nhất trước khi lấy `exp`: logits của mô hình thường nằm
/// quanh 20–30, mà `exp(30)` đã tràn khỏi khoảng an toàn của f32 khi cộng dồn
/// hàng vạn số hạng.
fn log_xac_suat(logits: &[f32], chi_so: usize) -> f32 {
    let Some(&muc_tieu) = logits.get(chi_so) else {
        return -20.0;
    };
    let lon_nhat = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let tong: f32 = logits.iter().map(|&x| (x - lon_nhat).exp()).sum();
    muc_tieu - lon_nhat - tong.ln()
}

impl ChamDiem for MoHinh {
    fn cham(&self, cau: &str) -> f32 {
        // Lỗi lúc chấm không được làm hỏng cả lượt xử lý sách. Trả điểm rất
        // thấp thì ứng viên ấy tự thua, và bản gốc — cũng chấm bằng chính hàm
        // này — thắng. Nói cách khác mô hình hỏng thì ứng dụng thoái về chế độ
        // không mô hình, chứ không sửa bừa.
        self.cham_that(cau).unwrap_or(f32::NEG_INFINITY)
    }

    fn cham_cho_trong(&self, truoc: &str, dien: &str, sau: &str) -> f32 {
        // Lỗi ở đây nguy hơn ở `cham`: một lượt lỗi có thể để lại bộ đệm KV dở
        // dang, và lượt sau tưởng tiền tố còn nguyên. Nên xoá dấu vết đệm trước
        // khi trả về, rồi mới để ứng viên ấy tự thua.
        match self.cham_cho_trong_that(truoc, dien, sau) {
            Ok(d) => d,
            Err(_) => {
                self.dem_tien_to.borrow_mut().clear();
                f32::NEG_INFINITY
            }
        }
    }
}

/// Kiểm nhanh xem file có phải GGUF không, để báo lỗi tử tế thay vì để
/// llama.cpp in ra một trang chữ đỏ.
pub fn la_gguf(duong_dan: &Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(duong_dan) else {
        return false;
    };
    let mut dau = [0u8; 4];
    f.read_exact(&mut dau).is_ok() && &dau == b"GGUF"
}

#[cfg(test)]
mod kiem {
    use super::*;

    #[test]
    fn log_softmax_khong_tran_so() {
        // Logits thật của mô hình hay nằm quanh 20–30. Tính thẳng `exp` rồi
        // cộng là tràn, mà tràn thì ra `inf` và mọi ứng viên bằng điểm nhau —
        // hỏng lặng lẽ, không lỗi nào bật ra.
        let l = vec![30.0f32, 29.0, 28.0, 1.0];
        let p = log_xac_suat(&l, 0);
        assert!(p.is_finite(), "tràn số");
        assert!(p < 0.0 && p > -3.0, "log-xác suất vô lý: {p}");
        // Tổng xác suất phải bằng 1.
        let tong: f32 = (0..l.len()).map(|i| log_xac_suat(&l, i).exp()).sum();
        assert!((tong - 1.0).abs() < 1e-4, "tổng xác suất {tong}");
    }

    #[test]
    fn chi_so_ngoai_bang_khong_no() {
        assert_eq!(log_xac_suat(&[1.0, 2.0], 99), -20.0);
    }
}
