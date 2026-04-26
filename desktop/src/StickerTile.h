#pragma once

#include <QFrame>
#include <QMovie>
#include <memory>

#include "RustBridge.h"

class QLabel;
class QPushButton;
class StickerTile : public QFrame
{
    Q_OBJECT

public:
    explicit StickerTile(
        const universal_stickers::StickerRecord& record,
        int tileWidth,
        int previewSize,
        QWidget* parent = nullptr
    );

signals:
    void activated(quint64 id);
    void deleteRequested(quint64 id, const QString& name);
    void editRequested(quint64 id, const QString& currentName);

protected:
    void mouseReleaseEvent(QMouseEvent* event) override;

private:
    void setupPreview();

    universal_stickers::StickerRecord m_record;
    QLabel* m_preview = nullptr;
    QLabel* m_name = nullptr;
    QPushButton* m_editButton = nullptr;
    QPushButton* m_deleteButton = nullptr;
    std::unique_ptr<QMovie> m_movie;
};
