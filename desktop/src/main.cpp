#include <QApplication>
#include <QMessageBox>
#include <QStandardPaths>

#include "MainWindow.h"
#include "RustBridge.h"

int main(int argc, char* argv[])
{
    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("Universal Stickers"));
    QApplication::setOrganizationName(QStringLiteral("UniversalStickers"));

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
