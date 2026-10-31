# Người Việt gõ tiếng Việt hiện nay: thói quen và phàn nàn

Ngày khảo sát tài liệu: 2026-10-31.

Đây là nghiên cứu định tính từ thảo luận công khai, không phải khảo sát đại diện
toàn quốc. Các bài viết được chọn vì có người dùng thật mô tả lỗi và nhu cầu cụ
thể, không phải để lấy con số thị phần chính xác.

> Quyết định hiện tại: **chưa ký số**. Các đề xuất về code signing dưới đây chỉ
> là backlog, không phải việc đang triển khai.

## 1. Người ta đang gõ kiểu gì?

- **Telex vẫn là lựa chọn phổ biến của số đông**, đặc biệt với người mới, người
  gõ chủ yếu tiếng Việt và người quen bàn phím điện thoại. Lý do được nhắc nhiều
  nhất: ít phải rời hàng phím chữ, gõ nhanh hơn.
- **VNI vẫn có nhóm người dùng trung thành**, thường là người làm việc song ngữ
  Việt/Anh, lập trình viên, người chơi game trên PC, hoặc người học VNI từ nhỏ.
  Lý do: gõ tiếng Anh ít bị đổi dấu, dấu tách khỏi chữ nên dễ kiểm soát.
- **Telex trên điện thoại, VNI trên PC** là kiểu kết hợp được nhắc khá nhiều.
- Không có số liệu đại diện hiện nay. Một khảo sát tự chọn trên Tinh tế năm 2022
  với 354 người cho thấy **64,7% dùng bộ gõ bên thứ ba**, phần còn lại dùng bộ gõ
  mặc định của Windows. Con số này chỉ nói về nhóm người đọc Tinh tế, không phải
  toàn bộ người dùng Việt Nam.

Nguồn:
- [Tinh tế: khảo sát bộ gõ tiếng Việt trên Windows](https://tinhte.vn/thread/ket-qua-khao-sat-bo-go-tieng-viet-tren-windows-hon-60-su-dung-bo-go-ben-thu-ba.3519894/)
- [Tinh tế: VNI hay Telex?](https://tinhte.vn/thread/anh-em-go-tieng-viet-tren-may-tinh-windows-theo-kieu-vni-hay-telex.3702027/)
- [GEARVN: ưu nhược điểm Telex và VNI](https://gearvn.com/blogs/cong-nghe/uu-nhuoc-diem-cua-telex-va-vni-anh-em-dang-dung-bo-go-nao)
- [Reddit r/VietNam: VNI hay Telex](https://www.reddit.com/r/VietNam/comments/149xtaj/do_you_prefer_vni_or_telex_for_typing/)

## 2. Các phàn nàn lặp lại nhiều nhất

### 2.1. Mất chữ, lặp chữ, nuốt dấu cách

Đây là nhóm lỗi bị phàn nàn nhiều nhất, và cũng là lỗi khó tái hiện nhất:

- Mất ký tự đầu tiên khi bắt đầu gõ, đặc biệt trên macOS/trình duyệt Chromium.
- Gõ địa chỉ web bị `đ` thành `dđ`, hoặc chữ bị lặp.
- Discord trên Windows 11 với bộ gõ mặc định bị nhân đôi ký tự.
- Bộ gõ mặc định Windows 11 làm hỏng số OTP/PIN, số tài khoản, số tiền.
- Trong game, phím kỹ năng (ví dụ `W` trong LMHT) bị biến thành `ư` hoặc bị chặn.
- Một số bộ gõ làm mất dấu cách khi gõ trong trình duyệt.

Nguồn:
- [Reddit: first letter disappears with VNI](https://www.reddit.com/r/VietNam/comments/w3zssj/first_letter_keeps_disappearing_when_typing_using/)
- [Reddit: Windows 11 Telex broken in Discord](https://www.reddit.com/r/discordapp/comments/13vl4c2/vietnamese_telex_texting_in_window_11_is_broken/)
- [VOZ: EVKey/OpenKey, lỗi game và trình duyệt](https://voz.vn/t/khuyen-cao-nen-bo-evkey-va-chuyen-qua-openkey.977921/page-14)
- [Tinh tế: khảo sát bộ gõ, phản hồi về OTP/số](https://tinhte.vn/thread/ket-qua-khao-sat-bo-go-tieng-viet-tren-windows-hon-60-su-dung-bo-go-ben-thu-ba.3519894/)

### 2.2. Gõ tiếng Anh và trộn Việt/Anh

Đây là lý do lớn khiến người dùng chọn VNI hoặc tắt bộ gõ:

- Telex dễ biến từ tiếng Anh thành từ tiếng Việt (`boots` → `bốt`, `as` → `á`).
- Người gõ Telex phải gõ lặp phím hoặc bật/tắt liên tục khi chuyển ngữ.
- VNI dễ gõ tiếng Anh hơn nhưng chậm hơn và bất tiện trên bàn phím nhỏ/không có
  hàng số.
- Người dùng làm việc trong VS Code, terminal, CAD, Office thường muốn tiếng Anh
  ở app này và tiếng Việt ở app khác.

Nguồn:
- [Tinh tế: VNI hay Telex](https://tinhte.vn/thread/anh-em-go-tieng-viet-tren-may-tinh-windows-theo-kieu-vni-hay-telex.3702027/)
- [GEARVN: Telex gõ tiếng Anh khó chịu](https://gearvn.com/blogs/cong-nghe/uu-nhuoc-diem-cua-telex-va-vni-anh-em-dang-dung-bo-go-nao)

### 2.3. Trạng thái V/E theo từng ứng dụng

Người dùng rất thích việc bộ gõ **nhớ trạng thái bật/tắt tiếng Việt cho từng
app**. UniKey bổ sung tính năng này từ 4.6 RC1 (2023); người dùng mô tả có thể
để Chrome/Zalo tiếng Việt, VS Code/game tiếng Anh, chuyển qua lại không cần bấm
lại.

Nguồn:
- [UniKey blog: 4.6 RC1, nhớ trạng thái theo ứng dụng](https://www.unikey.org/blog.html)
- [VOZ: thảo luận EVKey/OpenKey, app exclusion](https://voz.vn/t/khuyen-cao-nen-bo-evkey-va-chuyen-qua-openkey.977921/page-7)

### 2.4. Office, Excel, CAD, Adobe

- Bộ gõ bên thứ ba thường không đặt language tag của tài liệu Office, nên Word
  vẫn coi là tiếng Anh và không kiểm tra chính tả đúng.
- Bộ gõ mặc định Windows 11 hòa hợp với Office hơn nhưng lại vướng phím tắt trong
  CAD/PDF XChange, và hay làm hỏng số/OTP.
- Người dùng AutoCAD/Corel/Photoshop/Illustrator cần gõ tiếng Việt trực tiếp
  trong app; một số bộ gõ cũ không gõ được.
- TSF in-process (như Neokey) được người dùng khen vì gõ được trong Excel/AutoCAD,
  không có gạch chân, nhưng vẫn có lỗi phím tắt toàn cục, uninstall để lại language
  bar, và cần admin.

Nguồn:
- [VOZ page 7: Office language, CAD/PDF, bộ gõ mặc định](https://voz.vn/t/khuyen-cao-nen-bo-evkey-va-chuyen-qua-openkey.977921/page-7)
- [VOZ page 37: phản hồi Neokey TSF](https://voz.vn/t/khuyen-cao-nen-bo-evkey-va-chuyen-qua-openkey.977921/page-37)

### 2.5. Tin cậy, bảo mật, admin và antivirus

Đây là rào cản lớn với bộ gõ bên thứ ba:

- Người dùng lo phần mềm gõ phím giống keylogger, nhất là khi xin quyền admin.
- EVKey từng bị Windows Defender/ESET/Trend Micro báo nhầm là virus; tác giả nói
  chỉ cần sửa source là bị nhận diện, và code signing tốn khoảng 7 triệu
  đồng/năm.
- Người dùng khuyên chuyển sang open-source hoặc bộ gõ mặc định vì lý do an toàn.
- Một số bộ gõ "portable" vẫn cần admin để đăng ký IME/TSF.
- Uninstall không sạch, để lại language bar hoặc profile.
- Người dùng quan tâm đến ô mật khẩu: bộ gõ không nên xử lý/ghi lại mật khẩu.

Nguồn:
- [Genk: EVKey 6.0.4 bị antivirus báo nhầm](https://genk.vn/phan-mem-bo-go-tieng-viet-pho-bien-vua-ra-ban-moi-da-bi-windows-defender-nhan-dien-la-virus-tac-gia-noi-gi-165261706150146111.chn)
- [VOZ page 37: admin, password field, auto-update, uninstall](https://voz.vn/t/khuyen-cao-nen-bo-evkey-va-chuyen-qua-openkey.977921/page-37)

### 2.6. Gõ tắt/macro

- EVKey từng bị lỗi macro lặp chữ trong trình duyệt (`gg` → `Googoogle`,
  `VN` → `Việt NaViệt Namm`).
- OpenKey được khen là sửa được lỗi này; người dùng chuyển macro từ EVKey sang
  OpenKey khá dễ.
- Người dùng cần macro kiểu `@date`, `@time`, `@clipboard`, `@tab`, `@newline`.
  Trong VietFlow, `@clipboard` hiện đã được host Windows xử lý qua cache
  clipboard; TSF cũng xử lý khi macro được dùng trong app TSF.

Nguồn:
- [Tinh tế: chuyển EVKey sang OpenKey, lỗi macro trình duyệt](https://tinhte.vn/thread/minh-da-chuyen-tu-evkey-sang-openkey-trai-nghiem-tot-cho-ca-macos-va-windows.4009004/)

### 2.7. Bộ gõ mặc định Windows 11

Người dùng ngày càng dùng bộ gõ mặc định vì tiện và an toàn, nhưng vẫn phàn nàn:

- Không có gõ tắt, không có loại trừ ứng dụng.
- Xung đột phím tắt trong CAD/PDF/Excel.
- Lặp số/OTP, lặp ký tự trong Discord.
- Khó xóa bàn phím tiếng Việt khỏi language bar.
- Tự chuyển sang Telex dù người dùng muốn VNI.
- Sửa sai phải gõ lại cả từ.

Nguồn:
- [Tinh tế: khảo sát, phản hồi về bộ gõ mặc định](https://tinhte.vn/thread/ket-qua-khao-sat-bo-go-tieng-viet-tren-windows-hon-60-su-dung-bo-go-ben-thu-ba.3519894/)
- [VOZ: Windows tự chuyển sang Telex](https://voz.vn/t/windows-tu-dong-chuyen-sang-kieu-go-telex.434585/)
- [VOZ: bộ gõ phổ biến dừng phát triển, lỗi OTP/PIN](https://voz.vn/t/phan-mem-bo-go-tieng-viet-pho-bien-chinh-thuc-dung-phat-trien.1084445/page-2)

### 2.8. Thị trường đang trống chỗ

- EVKey ngừng phát triển từ 26/03/2025; người dùng đang tìm chỗ thay.
- OpenKey được khen open-source nhưng bị phàn nàn cũ, lỗi game, thiếu cập nhật.
- UniKey vẫn được tin dùng, có per-app state, ARM64 native (2025), code signing
  từ 2018, nhưng UI cũ và vẫn có lỗi tương thích app hiện đại.
- Neokey (TSF IME mới) đang được thử; người dùng khen Excel/AutoCAD nhưng còn
  lỗi phím tắt, uninstall, admin.

Nguồn:
- [Tinh tế: EVKey ngừng phát triển](https://tinhte.vn/thread/minh-da-chuyen-tu-evkey-sang-openkey-trai-nghiem-tot-cho-ca-macos-va-windows.4009004/)
- [UniKey blog](https://www.unikey.org/blog.html)
- [VOZ page 37: Neokey](https://voz.vn/t/khuyen-cao-nen-bo-evkey-va-chuyen-qua-openkey.977921/page-37)

## 3. VietFlow đã có gì?

VietFlow hiện đã đi đúng một số hướng:

- Telex, VNI, VIQR, Simple Telex, Telex + VNI, custom keymap.
- Unicode, Unicode tổ hợp (NFD), TCVN3, VNI Windows.
- Per-app profile cho kiểu gõ, bảng mã, cách gửi phím, macro.
- App exclusion, khóa phím tắt, temporary override.
- Auto-restore từ tiếng Anh, danh sách từ tiếng Anh, auto-capitalize.
- Macro opt-in, import/export nhiều định dạng.
- Portable, open source, ARM64, không cần runtime ngoài.
- Tray V/E rõ trạng thái, icon/font nhẹ.

## 4. Nên cải thiện gì, theo thứ tự

### P0 — Niềm tin và độ ổn định

1. **Code signing + SHA256 + reproducible build**
   - EVKey mất niềm tin vì antivirus báo nhầm; UniKey đã ký từ 2018.
   - Ký executable/installer, công bố hash, hướng dẫn kiểm tra.
   - Ghi rõ: không telemetry, không gửi nội dung gõ, updater chỉ tải manifest/bản
     cài đã ký.

2. **Bộ test hồi quy theo app thật**
   - Chrome/Edge address bar, Google search, Facebook, Zalo, Discord, Teams.
   - Word/Excel, VS Code/terminal, AutoCAD/Corel/Photoshop, một game.
   - Test chuỗi số/OTP/PIN, ký tự đầu, dấu cách, macro, undo.
   - Đây là cách biến các phàn nàn "mất chữ/lặp chữ" thành test case.

3. **Không mặc định xin admin**
   - Hook thường không cần admin; chỉ app elevated/game hoặc TSF mới cần.
   - Giải thích rõ khi nào cần, không bật admin mặc định.
   - Uninstall sạch: language profile, startup, tray, file tạm.

4. **An toàn với ô mật khẩu**
   - Không log, không lưu nội dung gõ.
   - Cân nhắc tạm dừng xử lý khi phát hiện ô mật khẩu (UIA/TSF), hoặc ít nhất
     ghi rõ giới hạn.

### P1 — Khác biệt rõ so với đối thủ

5. **Nhớ trạng thái V/E theo ứng dụng**
   - Thêm `language_mode: inherit/on/off` vào `AppProfile`.
   - UI: "Ngôn ngữ cho app này: Theo chung / Luôn tiếng Việt / Luôn tiếng Anh".
   - Đây là tính năng được người dùng UniKey 4.6 khen rõ rệt, và giải quyết đúng
     nhu cầu VS Code/CAD/game vs Chrome/Zalo.

6. **Preset app một chạm**
   - VS Code, Windows Terminal, Chrome, Zalo, Teams, Word/Excel, AutoCAD, Corel,
     Photoshop/Illustrator, LMHT/CS2.
   - Mỗi preset đặt sẵn method (Simple Telex cho game), charset, injection,
     macro, language mode.

7. **Nâng chất lượng auto-restore tiếng Anh**
   - Từ điển lớn hơn + từ người dùng + cụm từ, không chỉ danh sách ngắn.
   - Hiển thị trạng thái "đã khôi phục từ" hoặc cho undo ngay.
   - Tránh ép các từ ngắn dễ trùng (`me`, `the`, `test`).

8. **Macro trình duyệt và `@clipboard`**
   - Thêm regression test cho macro trong Chrome/Facebook/Google search.
   - Implement `@clipboard` ở host hoặc bỏ claim khỏi UI/README/docs.
   - Giữ tương thích import EVKey/OpenKey/UniKey.

### P2 — TSF/IME có kiểm soát

9. **TSF chỉ như lựa chọn cho Office/CAD**
   - Neokey cho thấy TSF gõ được trong Excel/AutoCAD, không gạch chân.
   - Nhưng phải xử lý: phím tắt toàn cục, uninstall language bar, admin, ô mật khẩu,
     auto-update.
   - Không bật mặc định; chỉ khi user chọn, và có nút gỡ sạch.

### P3 — UX nhỏ nhưng quan trọng

10. **Không tự thêm bàn phím tiếng Việt vào Windows**
    - UniKey 4.5 đã bỏ auto-add; người dùng rất ghét language bar thừa.
11. **Chọn Telex/VNI ngay lần đầu**
    - Hỏi hoặc gợi ý theo thói quen; đừng mặc định Telex mà không nói.
12. **Hiển thị trạng thái V/E rõ, không gây nhầm với bộ gõ khác**
    - Nếu phát hiện Windows Vietnamese IME đang bật, cảnh báo xung đột.

## 5. Kết luận

Có thể lấy phàn nàn của người dùng làm backlog rất cụ thể. Hai việc có tác động
lớn nhất và phù hợp với kiến trúc hiện tại của VietFlow là:

1. **Per-app language state + preset app** (giải quyết trộn Việt/Anh, game, CAD,
   Office).
2. **Độ tin cậy: code signing, test matrix theo app, không admin mặc định, uninstall
   sạch**.

TSF/IME nên để sau, và chỉ như lựa chọn cho Office/CAD, vì chi phí kỹ thuật và
niềm tin cao hơn nhiều so với hook hiện tại.
