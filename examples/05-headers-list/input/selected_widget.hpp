#pragma once

#include "shared_dependency.hpp"

class SelectedWidget {
public:
    SelectedWidget() = default;

    int Value() const {
        return SHARED_WIDGET_OFFSET + 7;
    }
};
