// VietFlow Qt UI - Theme tokens
// Licensed under GNU General Public License v3.0 (GPLv3)

#pragma once

#include <QColor>
#include <QObject>

class Theme : public QObject {
    Q_OBJECT

    Q_PROPERTY(QColor bg READ bg CONSTANT)
    Q_PROPERTY(QColor sidebar READ sidebar CONSTANT)
    Q_PROPERTY(QColor surface READ surface CONSTANT)
    Q_PROPERTY(QColor surfaceAlt READ surfaceAlt CONSTANT)
    Q_PROPERTY(QColor surfaceHover READ surfaceHover CONSTANT)
    Q_PROPERTY(QColor border READ border CONSTANT)
    Q_PROPERTY(QColor text READ text CONSTANT)
    Q_PROPERTY(QColor textDim READ textDim CONSTANT)
    Q_PROPERTY(QColor textFaint READ textFaint CONSTANT)
    Q_PROPERTY(QColor accent READ accent CONSTANT)
    Q_PROPERTY(QColor accentStrong READ accentStrong CONSTANT)
    Q_PROPERTY(QColor accentHover READ accentHover CONSTANT)
    Q_PROPERTY(QColor accentSoft READ accentSoft CONSTANT)
    Q_PROPERTY(QColor success READ success CONSTANT)
    Q_PROPERTY(QColor danger READ danger CONSTANT)
    Q_PROPERTY(int radius READ radius CONSTANT)
    Q_PROPERTY(int pagePadding READ pagePadding CONSTANT)

public:
    explicit Theme(QObject *parent = nullptr) : QObject(parent) {}

    QColor bg() const { return QColor("#14171c"); }
    QColor sidebar() const { return QColor("#191d24"); }
    QColor surface() const { return QColor("#1e232b"); }
    QColor surfaceAlt() const { return QColor("#232933"); }
    QColor surfaceHover() const { return QColor("#29303b"); }
    QColor border() const { return QColor("#2b323d"); }
    QColor text() const { return QColor("#e7ecf3"); }
    QColor textDim() const { return QColor("#98a2b3"); }
    QColor textFaint() const { return QColor("#6b7686"); }
    QColor accent() const { return QColor("#3b82f6"); }
    QColor accentStrong() const { return QColor("#2563eb"); }
    QColor accentHover() const { return QColor("#4f92ff"); }
    QColor accentSoft() const { return QColor("#1b2a44"); }
    QColor success() const { return QColor("#34d399"); }
    QColor danger() const { return QColor("#f87171"); }
    int radius() const { return 10; }
    int pagePadding() const { return 24; }
};
