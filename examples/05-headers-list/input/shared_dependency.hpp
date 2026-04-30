#pragma once

#define SHARED_WIDGET_OFFSET 100

class SharedDependency {
public:
    SharedDependency() = default;

    int Value() const {
        return SHARED_WIDGET_OFFSET;
    }
};
