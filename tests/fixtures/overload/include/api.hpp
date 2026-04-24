#pragma once

namespace clash {

class Widget {
public:
    Widget();

    int set(int value);
    int set(double value);
};

int add(int lhs, int rhs);
double add(double lhs, double rhs);

} // namespace clash
