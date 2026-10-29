// VietFlow Qt UI - Settings backend
// Licensed under GNU General Public License v3.0 (GPLv3)

#include "settingsbackend.h"

#include <QDir>
#include <QDesktopServices>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>
#include <QUrl>
#include <QVariant>

#include <algorithm>

// ---------------------------------------------------------------------------
// MacroListModel
// ---------------------------------------------------------------------------
MacroListModel::MacroListModel(QObject *parent) : QAbstractListModel(parent) {}

int MacroListModel::rowCount(const QModelIndex &parent) const {
    return parent.isValid() ? 0 : m_rows.size();
}

QVariant MacroListModel::data(const QModelIndex &index, int role) const {
    if (!index.isValid() || index.row() < 0 || index.row() >= m_rows.size()) {
        return QVariant();
    }

    const auto &row = m_rows.at(index.row());
    switch (role) {
    case TriggerRole:
        return row.first;
    case ExpansionRole:
        return row.second;
    default:
        return QVariant();
    }
}

QHash<int, QByteArray> MacroListModel::roleNames() const {
    return {
        { TriggerRole, "trigger" },
        { ExpansionRole, "expansion" },
    };
}

void MacroListModel::setRows(const QList<QPair<QString, QString>> &rows) {
    beginResetModel();
    m_rows = rows;
    std::sort(m_rows.begin(), m_rows.end(), [](const auto &a, const auto &b) {
        return a.first.compare(b.first, Qt::CaseInsensitive) < 0;
    });
    endResetModel();
}

void MacroListModel::upsert(const QString &trigger, const QString &expansion) {
    if (trigger.trimmed().isEmpty()) {
        return;
    }

    for (int i = 0; i < m_rows.size(); ++i) {
        if (m_rows[i].first.compare(trigger, Qt::CaseInsensitive) == 0) {
            m_rows[i].second = expansion;
            const QModelIndex idx = index(i, 0);
            emit dataChanged(idx, idx, { ExpansionRole });
            return;
        }
    }

    beginResetModel();
    m_rows.append({ trigger, expansion });
    std::sort(m_rows.begin(), m_rows.end(), [](const auto &a, const auto &b) {
        return a.first.compare(b.first, Qt::CaseInsensitive) < 0;
    });
    endResetModel();
}

void MacroListModel::removeAt(int row) {
    if (row < 0 || row >= m_rows.size()) {
        return;
    }
    beginRemoveRows(QModelIndex(), row, row);
    m_rows.removeAt(row);
    endRemoveRows();
}

// ---------------------------------------------------------------------------
// AppListModel
// ---------------------------------------------------------------------------
AppListModel::AppListModel(QObject *parent) : QAbstractListModel(parent) {}

int AppListModel::rowCount(const QModelIndex &parent) const {
    return parent.isValid() ? 0 : m_names.size();
}

QVariant AppListModel::data(const QModelIndex &index, int role) const {
    if (!index.isValid() || index.row() < 0 || index.row() >= m_names.size()) {
        return QVariant();
    }
    if (role == NameRole) {
        return m_names.at(index.row());
    }
    return QVariant();
}

QHash<int, QByteArray> AppListModel::roleNames() const {
    return { { NameRole, "name" } };
}

void AppListModel::setNames(const QStringList &names) {
    beginResetModel();
    m_names = names;
    endResetModel();
}

void AppListModel::appendUnique(const QString &name) {
    const QString trimmed = name.trimmed();
    if (trimmed.isEmpty()) {
        return;
    }
    for (const QString &existing : m_names) {
        if (existing.compare(trimmed, Qt::CaseInsensitive) == 0) {
            return;
        }
    }
    beginInsertRows(QModelIndex(), m_names.size(), m_names.size());
    m_names.append(trimmed);
    endInsertRows();
}

void AppListModel::removeAt(int row) {
    if (row < 0 || row >= m_names.size()) {
        return;
    }
    beginRemoveRows(QModelIndex(), row, row);
    m_names.removeAt(row);
    endRemoveRows();
}

// ---------------------------------------------------------------------------
// SettingsBackend
// ---------------------------------------------------------------------------
SettingsBackend::SettingsBackend(QObject *parent) : QObject(parent) {
    applyDefaults();
    load();
    setStatus(QStringLiteral("Sẵn sàng"));
}

QString SettingsBackend::configFilePath() {
#ifdef Q_OS_WIN
    const QString appData = qEnvironmentVariable("APPDATA");
    QDir dir(appData.isEmpty() ? QDir::homePath() : appData);
    return dir.filePath(QStringLiteral("VietFlow/vietflow_config.json"));
#else
    QDir dir(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation));
    return dir.filePath(QStringLiteral("vietflow_config.json"));
#endif
}

QString SettingsBackend::hotkeyToString(int index) {
    switch (index) {
    case 1:
        return QStringLiteral("AltZ");
    case 2:
        return QStringLiteral("CtrlSpace");
    case 3:
        return QStringLiteral("AltShift");
    default:
        return QStringLiteral("CtrlShift");
    }
}

int SettingsBackend::hotkeyFromString(const QString &mode) {
    if (mode == QStringLiteral("AltZ")) {
        return 1;
    }
    if (mode == QStringLiteral("CtrlSpace")) {
        return 2;
    }
    if (mode == QStringLiteral("AltShift")) {
        return 3;
    }
    return 0;
}

void SettingsBackend::applyDefaults() {
    m_options = {
        { QStringLiteral("enabled"), true },
        { QStringLiteral("method"), 0 },
        { QStringLiteral("charset"), 0 },
        { QStringLiteral("hotkeyIndex"), 0 },
        { QStringLiteral("startWithWindows"), false },
        { QStringLiteral("notificationSound"), true },
        { QStringLiteral("spellCheck"), true },
        { QStringLiteral("autoRestore"), true },
        { QStringLiteral("modernStyle"), false },
        { QStringLiteral("freeMarking"), true },
        { QStringLiteral("fixShiftLag"), true },
        { QStringLiteral("consonantShortcuts"), false },
        { QStringLiteral("autoCapitalize"), false },
        { QStringLiteral("useClipboard"), false },
        { QStringLiteral("enableMacro"), false },
        { QStringLiteral("allowMacroWhenDisabled"), false },
        { QStringLiteral("typingTimeout"), 3000 },
        { QStringLiteral("englishKeywords"), QString() },
        { QStringLiteral("foreignInitials"), QStringLiteral("fjwz") },
    };

    m_macros.setRows({
        { QStringLiteral("vf"), QStringLiteral("VietFlow") },
        { QStringLiteral("vn"), QStringLiteral("Việt Nam") },
        { QStringLiteral("chxhcnvn"), QStringLiteral("Cộng hòa Xã hội Chủ nghĩa Việt Nam") },
    });

    m_excludedApps.setNames({
        QStringLiteral("valorant.exe"),
        QStringLiteral("cs2.exe"),
        QStringLiteral("League of Legends.exe"),
        QStringLiteral("Overwatch.exe"),
        QStringLiteral("dota2.exe"),
    });
}

void SettingsBackend::load() {
    QFile file(configFilePath());
    if (!file.exists() || !file.open(QIODevice::ReadOnly)) {
        saveToDisk();
        return;
    }

    const QByteArray data = file.readAll();
    file.close();

    QJsonParseError error {};
    const QJsonDocument document = QJsonDocument::fromJson(data, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return;
    }

    const QJsonObject object = document.object();

    m_options[QStringLiteral("enabled")] = object.value(QStringLiteral("enabled")).toBool(true);
    m_options[QStringLiteral("method")] = object.value(QStringLiteral("method")).toInt(0);
    m_options[QStringLiteral("charset")] = object.value(QStringLiteral("charset")).toInt(0);
    m_options[QStringLiteral("hotkeyIndex")] =
        hotkeyFromString(object.value(QStringLiteral("hotkey_mode")).toString(QStringLiteral("CtrlShift")));
    m_options[QStringLiteral("startWithWindows")] =
        object.value(QStringLiteral("start_with_windows")).toBool(false);
    m_options[QStringLiteral("notificationSound")] =
        object.value(QStringLiteral("notification_sound")).toBool(true);
    m_options[QStringLiteral("spellCheck")] = object.value(QStringLiteral("spell_check")).toBool(true);
    m_options[QStringLiteral("autoRestore")] = object.value(QStringLiteral("auto_restore")).toBool(true);
    m_options[QStringLiteral("modernStyle")] = object.value(QStringLiteral("modern_style")).toBool(false);
    m_options[QStringLiteral("freeMarking")] = object.value(QStringLiteral("free_marking")).toBool(true);
    m_options[QStringLiteral("fixShiftLag")] = object.value(QStringLiteral("fix_shift_lag")).toBool(true);
    m_options[QStringLiteral("consonantShortcuts")] =
        object.value(QStringLiteral("consonant_shortcuts")).toBool(false);
    m_options[QStringLiteral("autoCapitalize")] =
        object.value(QStringLiteral("auto_capitalize")).toBool(false);
    m_options[QStringLiteral("useClipboard")] =
        object.value(QStringLiteral("use_clipboard_for_unicode")).toBool(false);
    m_options[QStringLiteral("enableMacro")] = object.value(QStringLiteral("enable_macro")).toBool(false);
    m_options[QStringLiteral("allowMacroWhenDisabled")] =
        object.value(QStringLiteral("allow_macro_when_disabled")).toBool(false);
    m_options[QStringLiteral("typingTimeout")] =
        object.value(QStringLiteral("typing_timeout_ms")).toInt(3000);

    QStringList keywords;
    const QJsonArray keywordArray = object.value(QStringLiteral("english_keywords")).toArray();
    for (const QJsonValue &value : keywordArray) {
        keywords.append(value.toString());
    }
    m_options[QStringLiteral("englishKeywords")] = keywords.join(QLatin1Char('\n'));
    m_options[QStringLiteral("foreignInitials")] =
        object.value(QStringLiteral("foreign_initials")).toString(QStringLiteral("fjwz"));

    QList<QPair<QString, QString>> macroRows;
    const QJsonObject macros = object.value(QStringLiteral("macros")).toObject();
    for (auto it = macros.constBegin(); it != macros.constEnd(); ++it) {
        macroRows.append({ it.key(), it.value().toString() });
    }
    if (macroRows.isEmpty()) {
        macroRows = {
            { QStringLiteral("vf"), QStringLiteral("VietFlow") },
            { QStringLiteral("vn"), QStringLiteral("Việt Nam") },
        };
    }
    m_macros.setRows(macroRows);

    QStringList apps;
    const QJsonArray excluded = object.value(QStringLiteral("excluded_apps")).toArray();
    for (const QJsonValue &value : excluded) {
        apps.append(value.toString());
    }
    m_excludedApps.setNames(apps);

    emit optionsChanged();
}

void SettingsBackend::saveToDisk() {
    QJsonObject object;
    object[QStringLiteral("enabled")] = m_options.value(QStringLiteral("enabled")).toBool();
    object[QStringLiteral("method")] = m_options.value(QStringLiteral("method")).toInt();
    object[QStringLiteral("charset")] = m_options.value(QStringLiteral("charset")).toInt();
    object[QStringLiteral("modern_style")] = m_options.value(QStringLiteral("modernStyle")).toBool();
    object[QStringLiteral("free_marking")] = m_options.value(QStringLiteral("freeMarking")).toBool();
    object[QStringLiteral("spell_check")] = m_options.value(QStringLiteral("spellCheck")).toBool();
    object[QStringLiteral("auto_restore")] = m_options.value(QStringLiteral("autoRestore")).toBool();
    object[QStringLiteral("fix_shift_lag")] = m_options.value(QStringLiteral("fixShiftLag")).toBool();
    object[QStringLiteral("consonant_shortcuts")] =
        m_options.value(QStringLiteral("consonantShortcuts")).toBool();
    object[QStringLiteral("auto_capitalize")] = m_options.value(QStringLiteral("autoCapitalize")).toBool();
    object[QStringLiteral("enable_macro")] = m_options.value(QStringLiteral("enableMacro")).toBool();
    object[QStringLiteral("allow_macro_when_disabled")] =
        m_options.value(QStringLiteral("allowMacroWhenDisabled")).toBool();
    object[QStringLiteral("use_clipboard_for_unicode")] =
        m_options.value(QStringLiteral("useClipboard")).toBool();
    object[QStringLiteral("start_with_windows")] =
        m_options.value(QStringLiteral("startWithWindows")).toBool();
    object[QStringLiteral("notification_sound")] =
        m_options.value(QStringLiteral("notificationSound")).toBool();
    object[QStringLiteral("typing_timeout_ms")] = m_options.value(QStringLiteral("typingTimeout")).toInt();
    object[QStringLiteral("hotkey_mode")] =
        hotkeyToString(m_options.value(QStringLiteral("hotkeyIndex")).toInt());

    QJsonArray keywords;
    const QString keywordText = m_options.value(QStringLiteral("englishKeywords")).toString();
    const QStringList keywordLines = keywordText.split(QLatin1Char('\n'), Qt::SkipEmptyParts);
    for (const QString &line : keywordLines) {
        const QString keyword = line.trimmed();
        if (!keyword.isEmpty()) {
            keywords.append(keyword);
        }
    }
    object[QStringLiteral("english_keywords")] = keywords;

    QString initials = m_options.value(QStringLiteral("foreignInitials")).toString().trimmed();
    if (initials.isEmpty()) {
        initials = QStringLiteral("fjwz");
    }
    object[QStringLiteral("foreign_initials")] = initials;

    QJsonArray apps;
    for (const QString &name : m_excludedApps.names()) {
        apps.append(name);
    }
    object[QStringLiteral("excluded_apps")] = apps;

    QJsonObject macros;
    for (const auto &row : m_macros.rows()) {
        macros[row.first] = row.second;
    }
    object[QStringLiteral("macros")] = macros;

    const QString path = configFilePath();
    QDir().mkpath(QFileInfo(path).absolutePath());

    QFile file(path);
    if (file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        file.write(QJsonDocument(object).toJson(QJsonDocument::Indented));
    }
}

void SettingsBackend::setStatus(const QString &text) {
    if (m_statusText == text) {
        return;
    }
    m_statusText = text;
    emit statusTextChanged();
}

void SettingsBackend::setOption(const QString &key, const QVariant &value) {
    if (!m_options.contains(key) || m_options.value(key) == value) {
        return;
    }
    m_options[key] = value;
    emit optionsChanged();
}

void SettingsBackend::save() {
    saveToDisk();
    setStatus(QStringLiteral("Đã lưu cấu hình."));
}

void SettingsBackend::resetDefaults() {
    applyDefaults();
    saveToDisk();
    emit optionsChanged();
    setStatus(QStringLiteral("Đã khôi phục cấu hình mặc định."));
}

void SettingsBackend::toggleMode() {
    const bool enabled = !m_options.value(QStringLiteral("enabled")).toBool();
    m_options[QStringLiteral("enabled")] = enabled;
    saveToDisk();
    emit optionsChanged();
    setStatus(enabled ? QStringLiteral("Đã chuyển sang Tiếng Việt.")
                      : QStringLiteral("Đã chuyển sang English."));
}

void SettingsBackend::addMacro(const QString &trigger, const QString &expansion) {
    if (trigger.trimmed().isEmpty() || expansion.isEmpty()) {
        return;
    }
    m_macros.upsert(trigger.trimmed(), expansion);
    saveToDisk();
    setStatus(QStringLiteral("Đã cập nhật bảng gõ tắt."));
}

void SettingsBackend::removeMacro(int row) {
    m_macros.removeAt(row);
    saveToDisk();
    setStatus(QStringLiteral("Đã xóa macro."));
}

void SettingsBackend::addExcludedApp(const QString &name) {
    if (name.trimmed().isEmpty()) {
        return;
    }
    m_excludedApps.appendUnique(name);
    saveToDisk();
    setStatus(QStringLiteral("Đã thêm ứng dụng vào Game Mode."));
}

void SettingsBackend::removeExcludedApp(int row) {
    m_excludedApps.removeAt(row);
    saveToDisk();
    setStatus(QStringLiteral("Đã xóa ứng dụng khỏi Game Mode."));
}

void SettingsBackend::openConfigFolder() const {
    const QString folder = QFileInfo(configFilePath()).absolutePath();
    QDesktopServices::openUrl(QUrl::fromLocalFile(folder));
}
