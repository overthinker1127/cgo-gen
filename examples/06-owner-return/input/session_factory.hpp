#pragma once

class ManagedSession {
public:
    explicit ManagedSession(int id) : id_(id) {}
    ~ManagedSession() = default;

    int Id() const {
        return id_;
    }

    void Reset(int id) {
        id_ = id;
    }

private:
    int id_;
};

class SessionFactory {
public:
    SessionFactory() = default;
    ~SessionFactory() = default;

    ManagedSession* CreateSession(int id) {
        return new ManagedSession(id);
    }

    ManagedSession* BorrowDefaultSession() {
        static ManagedSession session(0);
        return &session;
    }
};
