#pragma once

#include "../B/B.h"

class A {
public:
    A() = default;

    B child;

    B* Child() {
        return &child;
    }

    int ChildValue() const {
        return child.Value();
    }

    void SetChildValue(int value) {
        child.SetValue(value);
    }
};
