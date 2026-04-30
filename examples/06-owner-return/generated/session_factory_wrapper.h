#ifndef CGOWRAP_SESSION_FACTORY_WRAPPER_H
#define CGOWRAP_SESSION_FACTORY_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ManagedSessionHandle ManagedSessionHandle;
typedef struct SessionFactoryHandle SessionFactoryHandle;

ManagedSessionHandle* cgowrap_ManagedSession_new(int id);
void cgowrap_ManagedSession_delete(ManagedSessionHandle* self);
int cgowrap_ManagedSession_Id(const ManagedSessionHandle* self);
void cgowrap_ManagedSession_Reset(ManagedSessionHandle* self, int id);
SessionFactoryHandle* cgowrap_SessionFactory_new(void);
void cgowrap_SessionFactory_delete(SessionFactoryHandle* self);
ManagedSessionHandle* cgowrap_SessionFactory_CreateSession(SessionFactoryHandle* self, int id);
ManagedSessionHandle* cgowrap_SessionFactory_BorrowDefaultSession(SessionFactoryHandle* self);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_SESSION_FACTORY_WRAPPER_H */
