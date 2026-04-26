#include <QApplication>
#include <QIcon>
#include <QMessageBox>
#include <QPainter>
#include <QPixmap>
#include <QStandardPaths>
#include <QSvgRenderer>

#include "MainWindow.h"
#include "RustBridge.h"

namespace {

QIcon loadAppIcon()
{
    QSvgRenderer renderer(QStringLiteral(":/icons/icon.svg"));
    if (!renderer.isValid()) {
        return {};
    }

    QIcon icon;
    for (const int size : {16, 24, 32, 48, 64, 128, 256}) {
        QPixmap pixmap(size, size);
        pixmap.fill(Qt::transparent);

        QPainter painter(&pixmap);
        renderer.render(&painter);
        icon.addPixmap(pixmap);
    }

    return icon;
}

}

int main(int argc, char* argv[])
{
    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("Universal Stickers"));
    QApplication::setOrganizationName(QStringLiteral("UniversalStickers"));
    QApplication::setDesktopFileName(QStringLiteral("universal-stickers"));
    QApplication::setWindowIcon(loadAppIcon());

    try {
        const QString dataDir = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
        auto library = universal_stickers::init_library(rustStringFromQString(dataDir));

        MainWindow window(std::move(library));
        window.show();
        return app.exec();
    } catch (const std::exception& error) {
        QMessageBox::critical(
            nullptr,
            QStringLiteral("Universal Stickers"),
            QStringLiteral("Failed to initialize the sticker library:\n%1").arg(QString::fromUtf8(error.what()))
        );
    }

    return 1;
}
