// VietFlow Qt UI - Rust engine bridge (C ABI)
// Licensed under GNU General Public License v3.0 (GPLv3)

#include "enginebridge.h"

#include "vietflow_core.h"

#include <QByteArray>
#include <QClipboard>
#include <QGuiApplication>

QString EngineBridge::convert(const QString &text, int source, int target) const {
    const QByteArray input = text.toUtf8();

    const size_t needed = vf_convert_charset(
        reinterpret_cast<const uint8_t *>(input.constData()),
        static_cast<size_t>(input.size()),
        static_cast<uint8_t>(source),
        static_cast<uint8_t>(target),
        nullptr,
        0);

    if (needed == 0) {
        return QString();
    }

    QByteArray output(static_cast<int>(needed), Qt::Uninitialized);
    vf_convert_charset(
        reinterpret_cast<const uint8_t *>(input.constData()),
        static_cast<size_t>(input.size()),
        static_cast<uint8_t>(source),
        static_cast<uint8_t>(target),
        reinterpret_cast<uint8_t *>(output.data()),
        static_cast<size_t>(output.size()));

    return QString::fromUtf8(output);
}

QString EngineBridge::stripAccents(const QString &text) const {
    const QByteArray input = text.toUtf8();

    const size_t needed = vf_strip_accents(
        reinterpret_cast<const uint8_t *>(input.constData()),
        static_cast<size_t>(input.size()),
        nullptr,
        0);

    if (needed == 0) {
        return QString();
    }

    QByteArray output(static_cast<int>(needed), Qt::Uninitialized);
    vf_strip_accents(
        reinterpret_cast<const uint8_t *>(input.constData()),
        static_cast<size_t>(input.size()),
        reinterpret_cast<uint8_t *>(output.data()),
        static_cast<size_t>(output.size()));

    return QString::fromUtf8(output);
}

QString EngineBridge::convertClipboard(int source, int target) const {
    QClipboard *clipboard = QGuiApplication::clipboard();
    const QString text = clipboard->text();
    if (text.isEmpty()) {
        return QStringLiteral("Clipboard trống hoặc không thể truy cập.");
    }

    const QString converted = convert(text, source, target);
    clipboard->setText(converted);
    return QStringLiteral("Đã chuyển mã Clipboard thành công!");
}
