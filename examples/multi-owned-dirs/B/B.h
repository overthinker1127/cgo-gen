#pragma once

class B {
public:
    B() = default;

    int Value() const {
        return value_;
    }

    void SetValue(int value) {
        value_ = value;
    }

private:
    int value_ = 0;
};
