#pragma once

#include <QObject>
#include <QAbstractNativeEventFilter>

class HotkeyService : public QObject, public QAbstractNativeEventFilter
{
    Q_OBJECT

public:
    explicit HotkeyService(QObject* parent = nullptr);
    ~HotkeyService() override;

    bool isAvailable() const;
    QString backendName() const;
    bool nativeEventFilter(const QByteArray& eventType, void* message, qintptr* result) override;

signals:
    void activated();

private:
    void registerBackend();
    void unregisterBackend();

    bool m_available = false;
    QString m_backendName;
};

