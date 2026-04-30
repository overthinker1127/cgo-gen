#pragma once

class SelectedCounter {
public:
    SelectedCounter() = default;

    int Increment(int value) const {
        return value + 1;
    }
};
