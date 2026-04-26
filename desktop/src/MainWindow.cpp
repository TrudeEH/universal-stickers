#include "MainWindow.h"

#include "StickerTile.h"

#include <QDragEnterEvent>
#include <QDropEvent>
#include <QActionGroup>
#include <QFrame>
#include <QFileDialog>
#include <QFileInfo>
#include <QHBoxLayout>
#include <QGridLayout>
#include <QInputDialog>
#include <QLayoutItem>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QMenu>
#include <QMenuBar>
#include <QMimeData>
#include <QPushButton>
#include <QResizeEvent>
#include <QScrollArea>
#include <QSettings>
#include <QShowEvent>
#include <QSizePolicy>
#include <QStatusBar>
#include <QUrl>
#include <QVBoxLayout>
#include <algorithm>

namespace {

constexpr int kGridSpacing = 12;
constexpr int kGridMargin = 4;
constexpr int kDefaultDisplaySizeIndex = 1;
constexpr auto kDisplaySizeSettingKey = "ui/displaySizeIndex";

struct DisplaySizePreset {
    const char* label;
    int tileWidth;
    int previewSize;
};

constexpr DisplaySizePreset kDisplaySizePresets[] = {
    {"Compact", 150, 110},
    {"Comfortable", 180, 140},
    {"Large", 220, 180},
};

bool isSupportedImportPath(const QString& path)
{
    static const QStringList extensions = {
        QStringLiteral("png"),
        QStringLiteral("jpg"),
        QStringLiteral("jpeg"),
        QStringLiteral("bmp"),
        QStringLiteral("webp"),
        QStringLiteral("gif"),
    };

    const QString suffix = QFileInfo(path).suffix().toLower();
    return extensions.contains(suffix);
}

QStringList localImportPathsFromUrls(const QList<QUrl>& urls)
{
    QStringList paths;
    for (const QUrl& url : urls) {
        if (!url.isLocalFile()) {
            continue;
        }

        const QString path = url.toLocalFile();
        if (isSupportedImportPath(path)) {
            paths.push_back(path);
        }
    }
    return paths;
}

}

MainWindow::MainWindow(rust::Box<universal_stickers::StickerLibrary> library, QWidget* parent)
    : QMainWindow(parent)
    , m_library(std::move(library))
{
    QSettings settings;
    m_displaySizeIndex = std::clamp(
        settings.value(QLatin1StringView(kDisplaySizeSettingKey), kDefaultDisplaySizeIndex).toInt(),
        0,
        static_cast<int>(std::size(kDisplaySizePresets)) - 1
    );

    buildUi();

    connect(m_searchEdit, &QLineEdit::textChanged, this, &MainWindow::reloadItems);
    connect(m_addButton, &QPushButton::clicked, this, &MainWindow::importItems);
    connect(&m_hotkeyService, &HotkeyService::activated, this, &MainWindow::toggleWindow);

    reloadItems();

    statusBar()->showMessage(
        m_hotkeyService.isAvailable()
            ? QStringLiteral("Hotkey active via %1: Ctrl+Meta+Space").arg(m_hotkeyService.backendName())
            : QStringLiteral("Global hotkey unavailable"),
        5000);
}

void MainWindow::showEvent(QShowEvent* event)
{
    QMainWindow::showEvent(event);

    const int columns = calculateColumnCount();
    if (columns != m_currentColumns) {
        rebuildGrid();
    }
}

void MainWindow::resizeEvent(QResizeEvent* event)
{
    QMainWindow::resizeEvent(event);

    const int columns = calculateColumnCount();
    if (columns != m_currentColumns) {
        rebuildGrid();
    }
}

void MainWindow::dragEnterEvent(QDragEnterEvent* event)
{
    if (event->mimeData()->hasUrls() &&
        !localImportPathsFromUrls(event->mimeData()->urls()).isEmpty()) {
        event->acceptProposedAction();
        return;
    }

    event->ignore();
}

void MainWindow::dropEvent(QDropEvent* event)
{
    const QStringList paths = localImportPathsFromUrls(event->mimeData()->urls());
    if (paths.isEmpty()) {
        event->ignore();
        return;
    }

    importPaths(paths);
    event->acceptProposedAction();
}

void MainWindow::reloadItems()
{
    try {
        const rust::Vec<universal_stickers::StickerRecord> items =
            m_library->list_items(rustStringFromQString(m_searchEdit->text()));

        m_records.clear();
        m_records.reserve(items.size());
        for (const auto& item : items) {
            m_records.push_back(item);
        }

        rebuildGrid();
    } catch (const std::exception& error) {
        showRustError(QStringLiteral("loading stickers"), error);
    }
}

void MainWindow::importItems()
{
    const QStringList paths = QFileDialog::getOpenFileNames(
        this,
        QStringLiteral("Import Stickers"),
        QString(),
        QStringLiteral("Images (*.png *.jpg *.jpeg *.bmp *.webp *.gif)")
    );

    if (paths.isEmpty()) {
        return;
    }

    importPaths(paths);
}

void MainWindow::exportBackup()
{
    const QString targetDir = QFileDialog::getExistingDirectory(
        this,
        QStringLiteral("Choose Backup Destination")
    );
    if (targetDir.isEmpty()) {
        return;
    }

    try {
        const auto backupPath = m_library->export_backup(rustStringFromQString(targetDir));
        statusBar()->showMessage(QStringLiteral("Exported backup to %1").arg(qStringFromRust(backupPath)), 5000);
    } catch (const std::exception& error) {
        showRustError(QStringLiteral("exporting backup"), error);
    }
}

void MainWindow::importBackup()
{
    const QString sourceDir = QFileDialog::getExistingDirectory(
        this,
        QStringLiteral("Choose Backup Or Previous Installation")
    );
    if (sourceDir.isEmpty()) {
        return;
    }

    const auto reply = QMessageBox::question(
        this,
        QStringLiteral("Import Backup"),
        QStringLiteral("Import stickers from this backup or previous installation into the current library?")
    );
    if (reply != QMessageBox::Yes) {
        return;
    }

    try {
        const std::size_t importedCount = m_library->import_backup(rustStringFromQString(sourceDir));
        reloadItems();
        statusBar()->showMessage(
            QStringLiteral("Imported %1 sticker(s) from backup").arg(static_cast<qulonglong>(importedCount)),
            5000
        );
    } catch (const std::exception& error) {
        showRustError(QStringLiteral("importing backup"), error);
    }
}

void MainWindow::deleteAllItems()
{
    if (m_records.empty()) {
        statusBar()->showMessage(QStringLiteral("There are no stickers to delete"), 2500);
        return;
    }

    const auto reply = QMessageBox::warning(
        this,
        QStringLiteral("Delete All Stickers"),
        QStringLiteral("Delete all %1 stickers from the library? This cannot be undone.")
            .arg(static_cast<qulonglong>(m_records.size())),
        QMessageBox::Yes | QMessageBox::Cancel,
        QMessageBox::Cancel
    );
    if (reply != QMessageBox::Yes) {
        return;
    }

    try {
        const std::size_t deletedCount = m_library->delete_all_items();
        reloadItems();
        statusBar()->showMessage(
            QStringLiteral("Deleted %1 sticker(s)").arg(static_cast<qulonglong>(deletedCount)),
            5000
        );
    } catch (const std::exception& error) {
        showRustError(QStringLiteral("deleting all stickers"), error);
    }
}

void MainWindow::setDisplaySize(int index)
{
    if (index < 0 || index >= static_cast<int>(std::size(kDisplaySizePresets))) {
        return;
    }

    m_displaySizeIndex = index;
    QSettings().setValue(QLatin1StringView(kDisplaySizeSettingKey), m_displaySizeIndex);
    rebuildGrid();
}

void MainWindow::importPaths(const QStringList& paths)
{
    QStringList acceptedPaths;
    QStringList names;
    acceptedPaths.reserve(paths.size());
    names.reserve(paths.size());

    for (const QString& path : paths) {
        const QFileInfo fileInfo(path);
        bool accepted = false;
        const QString name = QInputDialog::getText(
            this,
            QStringLiteral("Sticker Name"),
            QStringLiteral("Name for %1").arg(fileInfo.fileName()),
            QLineEdit::Normal,
            fileInfo.completeBaseName(),
            &accepted
        );

        if (!accepted) {
            continue;
        }

        acceptedPaths.push_back(path);
        names.push_back(name.trimmed());
    }

    if (acceptedPaths.isEmpty()) {
        return;
    }

    try {
        m_library->import_items(rustVecFromQStringList(acceptedPaths), rustVecFromQStringList(names));
        reloadItems();
        statusBar()->showMessage(QStringLiteral("Imported %1 sticker(s)").arg(acceptedPaths.size()), 3000);
    } catch (const std::exception& error) {
        showRustError(QStringLiteral("importing stickers"), error);
    }
}

void MainWindow::editItem(quint64 id, const QString& currentName)
{
    bool accepted = false;
    const QString newName = QInputDialog::getText(
        this,
        QStringLiteral("Rename Sticker"),
        QStringLiteral("New name"),
        QLineEdit::Normal,
        currentName,
        &accepted
    );

    if (!accepted) {
        return;
    }

    try {
        const auto renamed = m_library->rename_item(id, rustStringFromQString(newName));
        reloadItems();
        statusBar()->showMessage(QStringLiteral("Renamed sticker to %1").arg(qStringFromRust(renamed.name)), 3000);
    } catch (const std::exception& error) {
        showRustError(QStringLiteral("renaming sticker"), error);
    }
}

void MainWindow::deleteItem(quint64 id, const QString& name)
{
    const auto reply = QMessageBox::question(
        this,
        QStringLiteral("Delete Sticker"),
        QStringLiteral("Delete \"%1\" from the library?").arg(name)
    );
    if (reply != QMessageBox::Yes) {
        return;
    }

    try {
        m_library->delete_item(id);
        reloadItems();
        statusBar()->showMessage(QStringLiteral("Deleted %1").arg(name), 3000);
    } catch (const std::exception& error) {
        showRustError(QStringLiteral("deleting sticker"), error);
    }
}

void MainWindow::copyItem(quint64 id)
{
    try {
        const auto item = m_library->get_item(id);
        const QString assetPath = qStringFromRust(item.asset_path);
        const bool isGif = qStringFromRust(item.kind) == QStringLiteral("gif");
        m_clipboardService.copyFileToClipboard(assetPath, isGif);
        statusBar()->showMessage(QStringLiteral("Copied %1 to clipboard").arg(qStringFromRust(item.name)), 2500);
    } catch (const std::exception& error) {
        showRustError(QStringLiteral("copying sticker"), error);
    }
}

void MainWindow::toggleWindow()
{
    showNormal();
    raise();
    activateWindow();
}

void MainWindow::buildUi()
{
    setWindowTitle(QStringLiteral("Universal Stickers"));
    resize(1040, 720);
    setAcceptDrops(true);

    auto* fileMenu = menuBar()->addMenu(QStringLiteral("File"));
    fileMenu->addAction(QStringLiteral("Add Stickers"), this, &MainWindow::importItems);
    fileMenu->addSeparator();
    fileMenu->addAction(QStringLiteral("Export Backup"), this, &MainWindow::exportBackup);
    fileMenu->addAction(QStringLiteral("Import Backup"), this, &MainWindow::importBackup);

    auto* libraryMenu = menuBar()->addMenu(QStringLiteral("Library"));
    libraryMenu->addAction(QStringLiteral("Delete All Stickers"), this, &MainWindow::deleteAllItems);

    auto* viewMenu = menuBar()->addMenu(QStringLiteral("View"));
    auto* sizeActionGroup = new QActionGroup(this);
    sizeActionGroup->setExclusive(true);
    for (int index = 0; index < static_cast<int>(std::size(kDisplaySizePresets)); ++index) {
        QAction* action = viewMenu->addAction(QString::fromUtf8(kDisplaySizePresets[index].label));
        action->setCheckable(true);
        action->setChecked(index == m_displaySizeIndex);
        sizeActionGroup->addAction(action);
        connect(action, &QAction::triggered, this, [this, index]() {
            setDisplaySize(index);
        });
    }

    auto* central = new QWidget(this);
    auto* rootLayout = new QVBoxLayout(central);
    rootLayout->setContentsMargins(16, 16, 16, 16);
    rootLayout->setSpacing(12);

    auto* headerLayout = new QHBoxLayout();
    headerLayout->setContentsMargins(0, 0, 0, 0);
    headerLayout->setSpacing(10);

    m_searchEdit = new QLineEdit(central);
    m_searchEdit->setPlaceholderText(QStringLiteral("Search stickers"));
    m_searchEdit->setMinimumWidth(260);
    m_searchEdit->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    headerLayout->addWidget(m_searchEdit, 1);

    m_addButton = new QPushButton(QStringLiteral("Add"), central);
    headerLayout->addWidget(m_addButton, 0);

    rootLayout->addLayout(headerLayout);

    m_scrollArea = new QScrollArea(central);
    m_scrollArea->setWidgetResizable(true);
    m_scrollArea->setFrameShape(QFrame::NoFrame);
    m_scrollArea->setAlignment(Qt::AlignTop);

    m_scrollContent = new QWidget(m_scrollArea);
    m_scrollContent->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Maximum);
    auto* gridWrapperLayout = new QHBoxLayout(m_scrollContent);
    gridWrapperLayout->setContentsMargins(0, 0, 0, 0);
    gridWrapperLayout->setSpacing(0);
    gridWrapperLayout->setAlignment(Qt::AlignTop);

    m_gridSpacerLeft = new QWidget(m_scrollContent);
    m_gridSpacerLeft->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
    gridWrapperLayout->addWidget(m_gridSpacerLeft, 1);

    m_gridContainer = new QWidget(m_scrollContent);
    m_gridContainer->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Maximum);
    m_gridLayout = new QGridLayout(m_gridContainer);
    m_gridLayout->setContentsMargins(4, 4, 4, 4);
    m_gridLayout->setHorizontalSpacing(12);
    m_gridLayout->setVerticalSpacing(12);
    m_gridLayout->setAlignment(Qt::AlignTop | Qt::AlignHCenter);
    gridWrapperLayout->addWidget(m_gridContainer, 0, Qt::AlignTop);

    m_gridSpacerRight = new QWidget(m_scrollContent);
    m_gridSpacerRight->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
    gridWrapperLayout->addWidget(m_gridSpacerRight, 1);

    m_scrollArea->setWidget(m_scrollContent);
    rootLayout->addWidget(m_scrollArea, 1);

    setCentralWidget(central);
}

int MainWindow::currentTileWidth() const
{
    return kDisplaySizePresets[m_displaySizeIndex].tileWidth;
}

int MainWindow::currentPreviewSize() const
{
    return kDisplaySizePresets[m_displaySizeIndex].previewSize;
}

int MainWindow::calculateColumnCount() const
{
    int availableWidth = 0;
    if (m_scrollArea != nullptr) {
        availableWidth = m_scrollArea->viewport()->width();
    }

    if (availableWidth <= 0 && centralWidget() != nullptr) {
        availableWidth = centralWidget()->width();
    }

    if (availableWidth <= 0) {
        availableWidth = width();
    }

    const int usableWidth = std::max(0, availableWidth - (2 * kGridMargin));
    return std::max(1, (usableWidth + kGridSpacing) / (currentTileWidth() + kGridSpacing));
}

void MainWindow::rebuildGrid()
{
    const int previousColumns = m_currentColumns;
    clearGrid();

    const int columns = calculateColumnCount();
    m_currentColumns = columns;

    for (int column = 0; column < std::max(previousColumns, columns); ++column) {
        m_gridLayout->setColumnStretch(column, 0);
    }

    if (m_records.empty()) {
        auto* emptyLabel = new QLabel(QStringLiteral("No stickers found. Import a few to get started."), m_gridContainer);
        emptyLabel->setAlignment(Qt::AlignCenter);
        m_gridLayout->addWidget(emptyLabel, 0, 0, 1, columns);
        m_gridContainer->setMinimumWidth(0);
        m_gridContainer->adjustSize();
        return;
    }

    for (int index = 0; index < static_cast<int>(m_records.size()); ++index) {
        const int row = index / columns;
        const int column = index % columns;

        auto* tile = new StickerTile(
            m_records[static_cast<std::size_t>(index)],
            currentTileWidth(),
            currentPreviewSize(),
            m_gridContainer
        );
        connect(tile, &StickerTile::activated, this, &MainWindow::copyItem);
        connect(tile, &StickerTile::editRequested, this, &MainWindow::editItem);
        connect(tile, &StickerTile::deleteRequested, this, &MainWindow::deleteItem);
        m_gridLayout->addWidget(tile, row, column, Qt::AlignTop | Qt::AlignHCenter);
    }

    const int contentWidth =
        (columns * currentTileWidth()) +
        ((columns - 1) * kGridSpacing) +
        (2 * kGridMargin);
    m_gridContainer->setMinimumWidth(contentWidth);
    m_gridContainer->adjustSize();
}

void MainWindow::clearGrid()
{
    while (QLayoutItem* item = m_gridLayout->takeAt(0)) {
        if (QWidget* widget = item->widget()) {
            widget->deleteLater();
        }
        delete item;
    }
}

void MainWindow::showRustError(const QString& action, const std::exception& error)
{
    QMessageBox::critical(this, QStringLiteral("Universal Stickers"), QStringLiteral("Error while %1:\n%2").arg(action, QString::fromUtf8(error.what())));
}
