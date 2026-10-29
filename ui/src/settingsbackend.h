// VietFlow Qt UI - Settings backend
// Licensed under GNU General Public License v3.0 (GPLv3)
//
// Reads and writes the same JSON file as the native Rust host:
//   Windows: %APPDATA%\VietFlow\vietflow_config.json
// The Qt process never handles keystrokes; it only edits configuration.

#pragma once

#include <QAbstractListModel>
#include <QList>
#include <QPair>
#include <QString>
#include <QStringList>
#include <QVariantMap>
#include <QObject>

class MacroListModel : public QAbstractListModel {
    Q_OBJECT

public:
    enum Role {
        TriggerRole = Qt::UserRole + 1,
        ExpansionRole
    };

    explicit MacroListModel(QObject *parent = nullptr);

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;

    void setRows(const QList<QPair<QString, QString>> &rows);
    void upsert(const QString &trigger, const QString &expansion);
    void removeAt(int row);
    QList<QPair<QString, QString>> rows() const { return m_rows; }

private:
    QList<QPair<QString, QString>> m_rows;
};

class AppListModel : public QAbstractListModel {
    Q_OBJECT

public:
    enum Role {
        NameRole = Qt::UserRole + 1
    };

    explicit AppListModel(QObject *parent = nullptr);

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;

    void setNames(const QStringList &names);
    void appendUnique(const QString &name);
    void removeAt(int row);
    QStringList names() const { return m_names; }

private:
    QStringList m_names;
};

class SettingsBackend : public QObject {
    Q_OBJECT

    Q_PROPERTY(QVariantMap options READ options NOTIFY optionsChanged)
    Q_PROPERTY(MacroListModel *macros READ macros CONSTANT)
    Q_PROPERTY(AppListModel *excludedApps READ excludedApps CONSTANT)
    Q_PROPERTY(QString statusText READ statusText NOTIFY statusTextChanged)
    Q_PROPERTY(QString configPath READ configPath CONSTANT)

public:
    explicit SettingsBackend(QObject *parent = nullptr);

    QVariantMap options() const { return m_options; }
    MacroListModel *macros() { return &m_macros; }
    AppListModel *excludedApps() { return &m_excludedApps; }
    QString statusText() const { return m_statusText; }
    QString configPath() const { return configFilePath(); }

    Q_INVOKABLE void setOption(const QString &key, const QVariant &value);
    Q_INVOKABLE void save();
    Q_INVOKABLE void resetDefaults();
    Q_INVOKABLE void toggleMode();
    Q_INVOKABLE void openConfigFolder() const;

    Q_INVOKABLE void addMacro(const QString &trigger, const QString &expansion);
    Q_INVOKABLE void removeMacro(int row);
    Q_INVOKABLE void addExcludedApp(const QString &name);
    Q_INVOKABLE void removeExcludedApp(int row);

signals:
    void optionsChanged();
    void statusTextChanged();

private:
    void applyDefaults();
    void load();
    void saveToDisk();
    void setStatus(const QString &text);
    static QString configFilePath();
    static QString hotkeyToString(int index);
    static int hotkeyFromString(const QString &mode);

    QVariantMap m_options;
    MacroListModel m_macros;
    AppListModel m_excludedApps;
    QString m_statusText;
};
