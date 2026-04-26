#pragma once

#include <QObject>

class ClipboardService : public QObject
{
    Q_OBJECT

public:
    explicit ClipboardService(QObject* parent = nullptr);
    void copyFileToClipboard(const QString& assetPath, bool isGif) const;
};

