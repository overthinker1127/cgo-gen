package generated

/*
#include <stdlib.h>
#include "session_factory_wrapper.h"
*/
import "C"

import "errors"

type ManagedSession struct {
    ptr *C.ManagedSessionHandle
    owned bool
    root *bool
}

func NewManagedSession(id int32) (*ManagedSession, error) {
    ptr := C.cgowrap_ManagedSession_new(C.int(id))
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedManagedSession(ptr), nil
}

func (m *ManagedSession) Close() {
    if m == nil || m.ptr == nil {
        return
    }
    if !m.owned {
        return
    }
    if m.root != nil {
        *m.root = true
    }
    C.cgowrap_ManagedSession_delete(m.ptr)
    m.ptr = nil
}

func newOwnedManagedSession(ptr *C.ManagedSessionHandle) *ManagedSession {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &ManagedSession{ptr: ptr, owned: true, root: root}
}

func newBorrowedManagedSession(ptr *C.ManagedSessionHandle, root *bool) *ManagedSession {
    if ptr == nil {
        return nil
    }
    return &ManagedSession{ptr: ptr, root: root}
}

func requireManagedSessionHandle(m *ManagedSession) *C.ManagedSessionHandle {
    if m == nil || m.ptr == nil {
        panic("ManagedSession handle is required but nil")
    }
    if m.root != nil && *m.root {
        panic("ManagedSession handle is closed")
    }
    return m.ptr
}

func optionalManagedSessionHandle(m *ManagedSession) *C.ManagedSessionHandle {
    if m == nil {
        return nil
    }
    if m.root != nil && *m.root {
        panic("ManagedSession handle is closed")
    }
    return m.ptr
}

func (m *ManagedSession) Id() int32 {
    if m == nil || m.ptr == nil {
        return 0
    }
    if m.root != nil && *m.root {
        panic("ManagedSession handle is closed")
    }
    return int32(C.cgowrap_ManagedSession_Id(m.ptr))
}

func (m *ManagedSession) Reset(id int32) {
    if m == nil || m.ptr == nil {
        return
    }
    if m.root != nil && *m.root {
        panic("ManagedSession handle is closed")
    }
    C.cgowrap_ManagedSession_Reset(m.ptr, C.int(id))
}

type SessionFactory struct {
    ptr *C.SessionFactoryHandle
    owned bool
    root *bool
}

func NewSessionFactory() (*SessionFactory, error) {
    ptr := C.cgowrap_SessionFactory_new()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedSessionFactory(ptr), nil
}

func (s *SessionFactory) Close() {
    if s == nil || s.ptr == nil {
        return
    }
    if !s.owned {
        return
    }
    if s.root != nil {
        *s.root = true
    }
    C.cgowrap_SessionFactory_delete(s.ptr)
    s.ptr = nil
}

func newOwnedSessionFactory(ptr *C.SessionFactoryHandle) *SessionFactory {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &SessionFactory{ptr: ptr, owned: true, root: root}
}

func newBorrowedSessionFactory(ptr *C.SessionFactoryHandle, root *bool) *SessionFactory {
    if ptr == nil {
        return nil
    }
    return &SessionFactory{ptr: ptr, root: root}
}

func requireSessionFactoryHandle(s *SessionFactory) *C.SessionFactoryHandle {
    if s == nil || s.ptr == nil {
        panic("SessionFactory handle is required but nil")
    }
    if s.root != nil && *s.root {
        panic("SessionFactory handle is closed")
    }
    return s.ptr
}

func optionalSessionFactoryHandle(s *SessionFactory) *C.SessionFactoryHandle {
    if s == nil {
        return nil
    }
    if s.root != nil && *s.root {
        panic("SessionFactory handle is closed")
    }
    return s.ptr
}

func (s *SessionFactory) CreateSession(id int32) *ManagedSession {
    if s == nil || s.ptr == nil {
        return nil
    }
    if s.root != nil && *s.root {
        panic("SessionFactory handle is closed")
    }
    raw := C.cgowrap_SessionFactory_CreateSession(s.ptr, C.int(id))
    if raw == nil {
        return nil
    }
    return &ManagedSession{ptr: raw, owned: true, root: new(bool)}
}

func (s *SessionFactory) BorrowDefaultSession() *ManagedSession {
    if s == nil || s.ptr == nil {
        return nil
    }
    if s.root != nil && *s.root {
        panic("SessionFactory handle is closed")
    }
    raw := C.cgowrap_SessionFactory_BorrowDefaultSession(s.ptr)
    if raw == nil {
        return nil
    }
    return newBorrowedManagedSession(raw, s.root)
}
