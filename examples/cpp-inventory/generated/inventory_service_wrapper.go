package generated

/*
#include <stdlib.h>
#include "inventory_service_wrapper.h"
*/
import "C"

import "errors"

type InventoryService struct {
    ptr *C.InventoryServiceHandle
    owned bool
    root *bool
}

func NewInventoryService() (*InventoryService, error) {
    ptr := C.cgowrap_InventoryService_new()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedInventoryService(ptr), nil
}

func (i *InventoryService) Close() {
    if i == nil || i.ptr == nil {
        return
    }
    if !i.owned {
        return
    }
    if i.root != nil {
        *i.root = true
    }
    C.cgowrap_InventoryService_delete(i.ptr)
    i.ptr = nil
}

func newOwnedInventoryService(ptr *C.InventoryServiceHandle) *InventoryService {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &InventoryService{ptr: ptr, owned: true, root: root}
}

func newBorrowedInventoryService(ptr *C.InventoryServiceHandle, root *bool) *InventoryService {
    if ptr == nil {
        return nil
    }
    return &InventoryService{ptr: ptr, root: root}
}

func requireInventoryServiceHandle(i *InventoryService) *C.InventoryServiceHandle {
    if i == nil || i.ptr == nil {
        panic("InventoryService handle is required but nil")
    }
    if i.root != nil && *i.root {
        panic("InventoryService handle is closed")
    }
    return i.ptr
}

func optionalInventoryServiceHandle(i *InventoryService) *C.InventoryServiceHandle {
    if i == nil {
        return nil
    }
    if i.root != nil && *i.root {
        panic("InventoryService handle is closed")
    }
    return i.ptr
}

func (i *InventoryService) LoadItem(id int32, out *InventoryItem) bool {
    if i == nil || i.ptr == nil {
        return false
    }
    if i.root != nil && *i.root {
        panic("InventoryService handle is closed")
    }
    var cArg1 *C.InventoryItemHandle
    if out == nil {
        panic("reference facade/model argument cannot be nil")
    }
    if out != nil {
        cArg1 = out.ptr
    }
    result := C.cgowrap_InventoryService_LoadItem(i.ptr, C.int32_t(id), cArg1)
    return bool(result)
}

func (i *InventoryService) NextItem(cursor *int32, out *InventoryItem) bool {
    if i == nil || i.ptr == nil {
        return false
    }
    if i.root != nil && *i.root {
        panic("InventoryService handle is closed")
    }
    if cursor == nil {
        panic("cursor reference is nil")
    }
    cArg0 := C.int32_t(*cursor)
    var cArg1 *C.InventoryItemHandle
    if out == nil {
        panic("reference facade/model argument cannot be nil")
    }
    if out != nil {
        cArg1 = out.ptr
    }
    result := C.cgowrap_InventoryService_NextItem(i.ptr, &cArg0, cArg1)
    *cursor = int32(cArg0)
    return bool(result)
}
