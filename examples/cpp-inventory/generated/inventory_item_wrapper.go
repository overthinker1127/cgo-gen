package generated

/*
#include <stdlib.h>
#include "inventory_item_wrapper.h"
*/
import "C"

import "errors"

import "unsafe"

type InventoryItem struct {
    ptr *C.InventoryItemHandle
    owned bool
    root *bool
}

func NewInventoryItem() (*InventoryItem, error) {
    ptr := C.cgowrap_InventoryItem_new()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedInventoryItem(ptr), nil
}

func (i *InventoryItem) Close() {
    if i == nil || i.ptr == nil {
        return
    }
    if !i.owned {
        return
    }
    if i.root != nil {
        *i.root = true
    }
    C.cgowrap_InventoryItem_delete(i.ptr)
    i.ptr = nil
}

func newOwnedInventoryItem(ptr *C.InventoryItemHandle) *InventoryItem {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &InventoryItem{ptr: ptr, owned: true, root: root}
}

func newBorrowedInventoryItem(ptr *C.InventoryItemHandle, root *bool) *InventoryItem {
    if ptr == nil {
        return nil
    }
    return &InventoryItem{ptr: ptr, root: root}
}

func requireInventoryItemHandle(i *InventoryItem) *C.InventoryItemHandle {
    if i == nil || i.ptr == nil {
        panic("InventoryItem handle is required but nil")
    }
    if i.root != nil && *i.root {
        panic("InventoryItem handle is closed")
    }
    return i.ptr
}

func optionalInventoryItemHandle(i *InventoryItem) *C.InventoryItemHandle {
    if i == nil {
        return nil
    }
    if i.root != nil && *i.root {
        panic("InventoryItem handle is closed")
    }
    return i.ptr
}

func (i *InventoryItem) Id() int32 {
    if i == nil || i.ptr == nil {
        return 0
    }
    if i.root != nil && *i.root {
        panic("InventoryItem handle is closed")
    }
    return int32(C.cgowrap_InventoryItem_Id(i.ptr))
}

func (i *InventoryItem) SetId(id int32) {
    if i == nil || i.ptr == nil {
        return
    }
    if i.root != nil && *i.root {
        panic("InventoryItem handle is closed")
    }
    C.cgowrap_InventoryItem_SetId(i.ptr, C.int32_t(id))
}

func (i *InventoryItem) Quantity() int32 {
    if i == nil || i.ptr == nil {
        return 0
    }
    if i.root != nil && *i.root {
        panic("InventoryItem handle is closed")
    }
    return int32(C.cgowrap_InventoryItem_Quantity(i.ptr))
}

func (i *InventoryItem) SetQuantity(quantity int32) {
    if i == nil || i.ptr == nil {
        return
    }
    if i.root != nil && *i.root {
        panic("InventoryItem handle is closed")
    }
    C.cgowrap_InventoryItem_SetQuantity(i.ptr, C.int32_t(quantity))
}

func (i *InventoryItem) Name() (string, error) {
    if i == nil || i.ptr == nil {
        return "", errors.New("facade receiver is nil")
    }
    if i.root != nil && *i.root {
        panic("InventoryItem handle is closed")
    }
    raw := C.cgowrap_InventoryItem_Name(i.ptr)
    if raw == nil {
        return "", errors.New("wrapper returned nil string")
    }
    return C.GoString(raw), nil
}

func (i *InventoryItem) SetName(name string) {
    if i == nil || i.ptr == nil {
        return
    }
    if i.root != nil && *i.root {
        panic("InventoryItem handle is closed")
    }
    cArg0 := C.CString(name)
    defer C.free(unsafe.Pointer(cArg0))
    C.cgowrap_InventoryItem_SetName(i.ptr, cArg0)
}
