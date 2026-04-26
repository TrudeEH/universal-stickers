#pragma once

#include <QByteArray>
#include <QString>
#include <QStringList>
#include <string>

#include "universal-stickers-ffi/src/lib.rs.h"

inline QString qStringFromRust(const rust::String& value)
{
    return QString::fromUtf8(value.data(), static_cast<int>(value.size()));
}

inline rust::String rustStringFromQString(const QString& value)
{
    const QByteArray utf8 = value.toUtf8();
    return rust::String(std::string(utf8.constData(), static_cast<std::size_t>(utf8.size())));
}

inline rust::Vec<rust::String> rustVecFromQStringList(const QStringList& values)
{
    rust::Vec<rust::String> result;
    result.reserve(values.size());
    for (const QString& value : values) {
        result.push_back(rustStringFromQString(value));
    }
    return result;
}
