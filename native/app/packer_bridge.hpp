#pragma once
// Thin Qt-facing wrapper around omnipack_core, exposed to QML.
// Mirrors what src/omnipack/logic.py's run_pack_logic does for a single
// hardcoded sample - a smoke test proving the Qt <-> core toolchain works,
// not a feature-complete bridge (that's a follow-up task).

#include <QObject>
#include <QString>
#include <qqml.h>

#include "omnipack/models.hpp"
#include "omnipack/multi_container.hpp"

class PackerBridge : public QObject {
    Q_OBJECT
    QML_ELEMENT

public:
    using QObject::QObject;

    Q_INVOKABLE QString packSample() const {
        using namespace omnipack;

        Container base("C1", 100, 100, 100);
        std::vector<Item> items = {
            Item("Box_1", 40, 40, 40, 10.0),
            Item("Box_2", 30, 30, 30, 5.0),
            Item("Box_3", 20, 20, 20, 2.0),
        };

        MultiContainerEngine engine(base);
        auto containers = engine.pack_all(items);

        QString out;
        for (const auto& c : containers) {
            out += QString("Container %1: %2 items, %3%% utilization\n")
                       .arg(QString::fromStdString(c.id))
                       .arg(c.items.size())
                       .arg(c.volume_utilization(), 0, 'f', 1);
            for (const auto& it : c.items) {
                out += QString("  %1 @ (%2, %3, %4)\n")
                           .arg(QString::fromStdString(it.id))
                           .arg(it.position.x, 0, 'f', 1)
                           .arg(it.position.y, 0, 'f', 1)
                           .arg(it.position.z, 0, 'f', 1);
            }
        }
        return out;
    }
};
