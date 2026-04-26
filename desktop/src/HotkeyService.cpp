#include "HotkeyService.h"

#include <QAction>
#include <QApplication>
#include <QKeySequence>

#ifdef HAS_KGLOBALACCEL
#include <KGlobalAccel>
#endif

#ifdef Q_OS_WIN
#include <windows.h>
#endif

namespace {

#ifdef Q_OS_WIN
constexpr int kHotkeyId = 0x5154;
#endif

}

HotkeyService::HotkeyService(QObject* parent)
    : QObject(parent)
{
    registerBackend();
}

HotkeyService::~HotkeyService()
{
    unregisterBackend();
}

bool HotkeyService::isAvailable() const
{
    return m_available;
}

QString HotkeyService::backendName() const
{
    return m_backendName;
}

bool HotkeyService::nativeEventFilter(const QByteArray& eventType, void* message, qintptr* result)
{
    Q_UNUSED(eventType);
    Q_UNUSED(result);

#ifdef Q_OS_WIN
    MSG* nativeMessage = static_cast<MSG*>(message);
    if (nativeMessage != nullptr && nativeMessage->message == WM_HOTKEY && nativeMessage->wParam == kHotkeyId) {
        emit activated();
        return true;
    }
#endif

    return false;
}

void HotkeyService::registerBackend()
{
#ifdef Q_OS_WIN
    if (RegisterHotKey(nullptr, kHotkeyId, MOD_CONTROL | MOD_WIN, VK_SPACE)) {
        qApp->installNativeEventFilter(this);
        m_available = true;
        m_backendName = QStringLiteral("RegisterHotKey");
    }
    return;
#endif

#ifdef HAS_KGLOBALACCEL
    auto* action = new QAction(this);
    action->setObjectName(QStringLiteral("toggle-picker"));
    action->setText(QStringLiteral("Toggle Universal Stickers"));
    connect(action, &QAction::triggered, this, &HotkeyService::activated);

    const QList<QKeySequence> shortcuts = {QKeySequence(QStringLiteral("Ctrl+Meta+Space"))};
    KGlobalAccel::self()->setDefaultShortcut(action, shortcuts);
    m_available = KGlobalAccel::self()->setShortcut(action, shortcuts, KGlobalAccel::NoAutoloading);
    if (m_available) {
        m_backendName = QStringLiteral("KGlobalAccel");
    }
    return;
#endif
}

void HotkeyService::unregisterBackend()
{
#ifdef Q_OS_WIN
    if (m_available) {
        qApp->removeNativeEventFilter(this);
        UnregisterHotKey(nullptr, kHotkeyId);
    }
#endif
}

