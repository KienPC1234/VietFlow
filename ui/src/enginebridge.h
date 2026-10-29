// VietFlow Qt UI - Rust engine bridge (C ABI)
// Licensed under GNU General Public License v3.0 (GPLv3)

#pragma once

#include <QObject>
#include <QString>

class EngineBridge : public QObject {
    Q_OBJECT

public:
    explicit EngineBridge(QObject *parent = nullptr) : QObject(parent) {}

    Q_INVOKABLE QString convert(const QString &text, int source, int target) const;
    Q_INVOKABLE QString stripAccents(const QString &text) const;
    Q_INVOKABLE QString convertClipboard(int source, int target) const;
};
