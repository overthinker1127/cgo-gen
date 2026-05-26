package generated

/*
#include <stdlib.h>
#include "selected_counter_wrapper.h"
*/
import "C"

import "errors"

type SelectedCounter struct {
    ptr *C.SelectedCounterHandle
    owned bool
    root *bool
}

func NewSelectedCounter() (*SelectedCounter, error) {
    ptr := C.cgowrap_SelectedCounter_new()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedSelectedCounter(ptr), nil
}

func (s *SelectedCounter) Close() {
    if s == nil || s.ptr == nil {
        return
    }
    if !s.owned {
        return
    }
    if s.root != nil {
        *s.root = true
    }
    C.cgowrap_SelectedCounter_delete(s.ptr)
    s.ptr = nil
}

func newOwnedSelectedCounter(ptr *C.SelectedCounterHandle) *SelectedCounter {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &SelectedCounter{ptr: ptr, owned: true, root: root}
}

func newBorrowedSelectedCounter(ptr *C.SelectedCounterHandle, root *bool) *SelectedCounter {
    if ptr == nil {
        return nil
    }
    return &SelectedCounter{ptr: ptr, root: root}
}

func requireSelectedCounterHandle(s *SelectedCounter) *C.SelectedCounterHandle {
    if s == nil || s.ptr == nil {
        panic("SelectedCounter handle is required but nil")
    }
    if s.root != nil && *s.root {
        panic("SelectedCounter handle is closed")
    }
    return s.ptr
}

func optionalSelectedCounterHandle(s *SelectedCounter) *C.SelectedCounterHandle {
    if s == nil {
        return nil
    }
    if s.root != nil && *s.root {
        panic("SelectedCounter handle is closed")
    }
    return s.ptr
}

func (s *SelectedCounter) Increment(value int32) int32 {
    if s == nil || s.ptr == nil {
        return 0
    }
    if s.root != nil && *s.root {
        panic("SelectedCounter handle is closed")
    }
    return int32(C.cgowrap_SelectedCounter_Increment(s.ptr, C.int(value)))
}
