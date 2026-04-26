#include "ClipboardService.h"

#include <QClipboard>
#include <QGuiApplication>
#include <QImage>
#include <QImageReader>
#include <QMimeData>
#include <QUrl>

ClipboardService::ClipboardService(QObject* parent)
    : QObject(parent)
{
}

void ClipboardService::copyFileToClipboard(const QString& assetPath, bool isGif) const
{
    auto* mimeData = new QMimeData();
    mimeData->setUrls({QUrl::fromLocalFile(assetPath)});
    mimeData->setText(assetPath);

    QImageReader reader(assetPath);
    if (reader.canRead()) {
        const QImage preview = reader.read();
        if (!preview.isNull()) {
            mimeData->setImageData(preview);
        }
    }

    if (isGif) {
        mimeData->setData("application/x-universal-stickers-kind", QByteArrayLiteral("gif"));
    }

    if (QClipboard* clipboard = QGuiApplication::clipboard()) {
        clipboard->setMimeData(mimeData);
    }
}

