#include "session_factory_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "session_factory.hpp"

ManagedSessionHandle* cgowrap_ManagedSession_new(int id) {
    return reinterpret_cast<ManagedSessionHandle*>(new ManagedSession(id));
}

void cgowrap_ManagedSession_delete(ManagedSessionHandle* self) {
    delete reinterpret_cast<ManagedSession*>(self);
}

int cgowrap_ManagedSession_Id(const ManagedSessionHandle* self) {
    return reinterpret_cast<const ManagedSession*>(self)->Id();
}

void cgowrap_ManagedSession_Reset(ManagedSessionHandle* self, int id) {
    reinterpret_cast<ManagedSession*>(self)->Reset(id);
}

SessionFactoryHandle* cgowrap_SessionFactory_new(void) {
    return reinterpret_cast<SessionFactoryHandle*>(new SessionFactory());
}

void cgowrap_SessionFactory_delete(SessionFactoryHandle* self) {
    delete reinterpret_cast<SessionFactory*>(self);
}

ManagedSessionHandle* cgowrap_SessionFactory_CreateSession(SessionFactoryHandle* self, int id) {
    return reinterpret_cast<ManagedSessionHandle*>(reinterpret_cast<SessionFactory*>(self)->CreateSession(id));
}

ManagedSessionHandle* cgowrap_SessionFactory_BorrowDefaultSession(SessionFactoryHandle* self) {
    return reinterpret_cast<ManagedSessionHandle*>(reinterpret_cast<SessionFactory*>(self)->BorrowDefaultSession());
}
