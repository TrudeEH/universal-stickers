#pragma once

#include <QMainWindow>

#include <exception>
#include <vector>

#include "ClipboardService.h"
#include "HotkeyService.h"
#include "RustBridge.h"

class QGridLayout;
class QLineEdit;
class QPushButton;
class QScrollArea;
class QWidget;
class QDragEnterEvent;
class QDropEvent;

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(rust::Box<universal_stickers::StickerLibrary> library, QWidget* parent = nullptr);
    ~MainWindow() override = default;

protected:
    void showEvent(QShowEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    void dragEnterEvent(QDragEnterEvent* event) override;
    void dropEvent(QDropEvent* event) override;

private slots:
    void reloadItems();
    void importItems();
    void exportBackup();
    void importBackup();
    void deleteAllItems();
    void setDisplaySize(int index);
    void deleteItem(quint64 id, const QString& name);
    void editItem(quint64 id, const QString& currentName);
    void copyItem(quint64 id);
    void toggleWindow();

private:
    void buildUi();
    int currentTileWidth() const;
    int currentPreviewSize() const;
    int calculateColumnCount() const;
    void rebuildGrid();
    void clearGrid();
    void importPaths(const QStringList& paths);
    void showRustError(const QString& action, const std::exception& error);

    rust::Box<universal_stickers::StickerLibrary> m_library;
    ClipboardService m_clipboardService;
    HotkeyService m_hotkeyService;
    QLineEdit* m_searchEdit = nullptr;
    QPushButton* m_addButton = nullptr;
    QScrollArea* m_scrollArea = nullptr;
    QWidget* m_scrollContent = nullptr;
    QWidget* m_gridSpacerLeft = nullptr;
    QWidget* m_gridSpacerRight = nullptr;
    QWidget* m_gridContainer = nullptr;
    QGridLayout* m_gridLayout = nullptr;
    std::vector<universal_stickers::StickerRecord> m_records;
    int m_displaySizeIndex = 1;
    int m_currentColumns = 0;
};
