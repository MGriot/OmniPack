qt_policy(SET QTP0001 NEW)

qt_add_executable(omnipack_app main.cpp packer_bridge.hpp)

qt_add_qml_module(omnipack_app
    URI OmniPackApp
    VERSION 1.0
    QML_FILES Main.qml
    SOURCES packer_bridge.hpp
)

target_link_libraries(omnipack_app PRIVATE
    omnipack_core
    Qt6::Quick
)
