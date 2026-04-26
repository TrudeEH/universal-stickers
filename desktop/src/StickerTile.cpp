#include "StickerTile.h"

#include <QLabel>
#include <QHBoxLayout>
#include <QMouseEvent>
#include <QMovie>
#include <QPushButton>
#include <QSizePolicy>
#include <QStyle>
#include <QVBoxLayout>

StickerTile::StickerTile(
    const universal_stickers::StickerRecord& record,
    int tileWidth,
    int previewSize,
    QWidget* parent
)
    : QFrame(parent)
    , m_record(record)
{
    setFrameShape(QFrame::StyledPanel);
    setCursor(Qt::PointingHandCursor);
    setObjectName(QStringLiteral("stickerTile"));
    setStyleSheet(QStringLiteral(
        "#stickerTile { border: 1px solid palette(mid); border-radius: 10px; background: palette(base); }"
        "#stickerTile:hover { border-color: palette(highlight); }"
    ));
    setFixedWidth(tileWidth);
    setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Fixed);

    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(10, 10, 10, 10);
    layout->setSpacing(8);

    auto* actionsLayout = new QHBoxLayout();
    actionsLayout->setContentsMargins(0, 0, 0, 0);
    actionsLayout->setSpacing(6);

    m_editButton = new QPushButton(QStringLiteral("Edit"), this);
    m_editButton->setFlat(true);
    m_editButton->setCursor(Qt::ArrowCursor);
    m_editButton->setToolTip(QStringLiteral("Rename sticker"));
    connect(m_editButton, &QPushButton::clicked, this, [this]() {
        emit editRequested(m_record.id, qStringFromRust(m_record.name));
    });
    actionsLayout->addWidget(m_editButton, 0, Qt::AlignLeft);

    m_deleteButton = new QPushButton(this);
    m_deleteButton->setFlat(true);
    m_deleteButton->setCursor(Qt::ArrowCursor);
    m_deleteButton->setIcon(style()->standardIcon(QStyle::SP_DockWidgetCloseButton));
    m_deleteButton->setToolTip(QStringLiteral("Delete sticker"));
    connect(m_deleteButton, &QPushButton::clicked, this, [this]() {
        emit deleteRequested(m_record.id, qStringFromRust(m_record.name));
    });
    actionsLayout->addWidget(m_deleteButton, 0, Qt::AlignRight);

    layout->addLayout(actionsLayout);

    m_preview = new QLabel(this);
    m_preview->setAlignment(Qt::AlignCenter);
    m_preview->setMinimumSize(previewSize, previewSize);
    m_preview->setMaximumSize(previewSize, previewSize);
    layout->addWidget(m_preview, 0, Qt::AlignCenter);

    m_name = new QLabel(qStringFromRust(m_record.name), this);
    m_name->setAlignment(Qt::AlignCenter);
    m_name->setWordWrap(true);
    layout->addWidget(m_name);

    setupPreview();
}

void StickerTile::mouseReleaseEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) {
        emit activated(m_record.id);
    }
    QFrame::mouseReleaseEvent(event);
}

void StickerTile::setupPreview()
{
    const QString assetPath = qStringFromRust(m_record.asset_path);
    const QString thumbPath = qStringFromRust(m_record.thumb_path);

    if (qStringFromRust(m_record.kind) == QStringLiteral("gif")) {
        m_movie = std::make_unique<QMovie>(assetPath);
        m_movie->setScaledSize(m_preview->maximumSize());
        m_preview->setMovie(m_movie.get());
        m_movie->start();
        return;
    }

    QPixmap pixmap(thumbPath);
    if (!pixmap.isNull()) {
        m_preview->setPixmap(
            pixmap.scaled(m_preview->maximumSize(), Qt::KeepAspectRatio, Qt::SmoothTransformation)
        );
    } else {
        m_preview->setText(QStringLiteral("No preview"));
    }
}
