#pragma once

class OverloadMath {
public:
    OverloadMath()
        : base_(0) {}

    explicit OverloadMath(int base)
        : base_(base) {}

    ~OverloadMath() = default;

    double Add(int lhs, int rhs) {
        return static_cast<double>(base_ + lhs + rhs);
    }

    double Add(double lhs, double rhs) {
        return static_cast<double>(base_) + lhs + rhs;
    }

private:
    int base_;
};
