// VietFlow Qt UI - Application entry point
// Licensed under GNU General Public License v3.0 (GPLv3)
//
// The UI is a pure configuration client. Keystroke handling stays in the native
// Rust host; this process only talks to the core through the C ABI when the
// converter needs charset translation.

#include <QFont>
#include <QGuiApplication>
#include <QIcon>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QUrl>

#include "enginebridge.h"
#include "settingsbackend.h"
#include "theme.h"

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("VietFlow"));
    app.setApplicationVersion(QStringLiteral("0.1.0-alpha"));
    app.setOrganizationName(QStringLiteral("VietFlow"));
    app.setWindowIcon(QIcon(QStringLiteral(":/assets/app-logo.png")));

    // Predictable cross-platform styling: Basic controls + our own theme.
    QQuickStyle::setStyle(QStringLiteral("Basic"));

    QFont font(QStringLiteral("Segoe UI"));
    font.setPointSize(10);
    app.setFont(font);

    SettingsBackend backend;
    EngineBridge engine;
    Theme theme;

    int initialPage = 0;
    const QStringList args = app.arguments();
    for (int i = 1; i < args.size(); ++i) {
        if (args.at(i) == QStringLiteral("--convert")) {
            initialPage = 4;
        } else if (args.at(i) == QStringLiteral("--page") && i + 1 < args.size()) {
            bool ok = false;
            const int page = args.at(++i).toInt(&ok);
            if (ok) {
                initialPage = qBound(0, page, 6);
            }
        }
    }

    QQmlApplicationEngine qml;
    qml.rootContext()->setContextProperty(QStringLiteral("backend"), &backend);
    qml.rootContext()->setContextProperty(QStringLiteral("engine"), &engine);
    qml.rootContext()->setContextProperty(QStringLiteral("theme"), &theme);
    qml.rootContext()->setContextProperty(QStringLiteral("initialPage"), initialPage);

    qml.load(QUrl(QStringLiteral("qrc:/qml/Main.qml")));
    if (qml.rootObjects().isEmpty()) {
        return -1;
    }

    return app.exec();
}
