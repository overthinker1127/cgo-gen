#pragma once

class Vector2 {
public:
    Vector2() : x_(0), y_(0) {}
    Vector2(int x, int y) : x_(x), y_(y) {}

    int X() const { return x_; }
    int Y() const { return y_; }

    Vector2 operator+(const Vector2& rhs) const {
        return Vector2(x_ + rhs.x_, y_ + rhs.y_);
    }

    bool operator==(const Vector2& rhs) const {
        return x_ == rhs.x_ && y_ == rhs.y_;
    }

    operator bool() const {
        return x_ != 0 || y_ != 0;
    }

private:
    int x_;
    int y_;
};

inline Vector2 operator-(const Vector2& lhs, const Vector2& rhs) {
    return Vector2(lhs.X() - rhs.X(), lhs.Y() - rhs.Y());
}
