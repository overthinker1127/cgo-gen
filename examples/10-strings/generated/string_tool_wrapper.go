package generated

/*
#include <stdlib.h>
#include "string_tool_wrapper.h"
*/
import "C"

import "errors"

import "unsafe"

type StringTool struct {
    ptr *C.StringToolHandle
    owned bool
    root *bool
}

func NewStringTool() (*StringTool, error) {
    ptr := C.cgowrap_StringTool_new__void()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedStringTool(ptr), nil
}

func NewStringToolWithPrefix(prefix string) (*StringTool, error) {
    cArg0 := C.CString(prefix)
    defer C.free(unsafe.Pointer(cArg0))
    ptr := C.cgowrap_StringTool_new__c_str(cArg0)
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedStringTool(ptr), nil
}

func (s *StringTool) Close() {
    if s == nil || s.ptr == nil {
        return
    }
    if !s.owned {
        return
    }
    if s.root != nil {
        *s.root = true
    }
    C.cgowrap_StringTool_delete(s.ptr)
    s.ptr = nil
}

func newOwnedStringTool(ptr *C.StringToolHandle) *StringTool {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &StringTool{ptr: ptr, owned: true, root: root}
}

func newBorrowedStringTool(ptr *C.StringToolHandle, root *bool) *StringTool {
    if ptr == nil {
        return nil
    }
    return &StringTool{ptr: ptr, root: root}
}

func requireStringToolHandle(s *StringTool) *C.StringToolHandle {
    if s == nil || s.ptr == nil {
        panic("StringTool handle is required but nil")
    }
    if s.root != nil && *s.root {
        panic("StringTool handle is closed")
    }
    return s.ptr
}

func optionalStringToolHandle(s *StringTool) *C.StringToolHandle {
    if s == nil {
        return nil
    }
    if s.root != nil && *s.root {
        panic("StringTool handle is closed")
    }
    return s.ptr
}

func (s *StringTool) Prefix() (string, error) {
    if s == nil || s.ptr == nil {
        return "", errors.New("facade receiver is nil")
    }
    if s.root != nil && *s.root {
        panic("StringTool handle is closed")
    }
    raw := C.cgowrap_StringTool_Prefix(s.ptr)
    if raw == nil {
        return "", errors.New("wrapper returned nil string")
    }
    return C.GoString(raw), nil
}

func (s *StringTool) SetPrefix(prefix string) {
    if s == nil || s.ptr == nil {
        return
    }
    if s.root != nil && *s.root {
        panic("StringTool handle is closed")
    }
    cArg0 := C.CString(prefix)
    defer C.free(unsafe.Pointer(cArg0))
    C.cgowrap_StringTool_SetPrefix(s.ptr, cArg0)
}

func (s *StringTool) Join(value string) (string, error) {
    if s == nil || s.ptr == nil {
        return "", errors.New("facade receiver is nil")
    }
    if s.root != nil && *s.root {
        panic("StringTool handle is closed")
    }
    cArg0 := C.CString(value)
    defer C.free(unsafe.Pointer(cArg0))
    raw := C.cgowrap_StringTool_Join(s.ptr, cArg0)
    if raw == nil {
        return "", errors.New("wrapper returned nil string")
    }
    defer C.cgowrap_string_free(raw)
    return C.GoString(raw), nil
}

func (s *StringTool) EchoView(value string) (string, error) {
    if s == nil || s.ptr == nil {
        return "", errors.New("facade receiver is nil")
    }
    if s.root != nil && *s.root {
        panic("StringTool handle is closed")
    }
    cArg0 := C.CString(value)
    defer C.free(unsafe.Pointer(cArg0))
    raw := C.cgowrap_StringTool_EchoView(s.ptr, cArg0)
    if raw == nil {
        return "", errors.New("wrapper returned nil string")
    }
    defer C.cgowrap_string_free(raw)
    return C.GoString(raw), nil
}
