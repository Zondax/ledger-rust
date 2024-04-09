#ifndef WRAPPERFS_H_
#define WRAPPERFS_H_

#include "defs.h"

// Taken from Makefile.defines
#define IO_SEPROXYHAL_BUFFER_SIZE_B 300

#define BAGL_WIDTH 128
#define BAGL_HEIGHT 32
#define HAVE_BAGL_FONT_INTER_REGULAR_24PX
#define HAVE_BAGL_FONT_INTER_SEMIBOLD_24PX
#define HAVE_BAGL_FONT_INTER_REGULAR_32PX
#define HAVE_BAGL_FONT_HMALPHAMONO_MEDIUM_32PX


#define HAVE_PIEZO_SOUND
#define HAVE_SE_TOUCH
#define HAVE_BLE
#define HAVE_BLE_APDU
#define NBGL_QRCODE


#define NBGL_PAGE
#define NBGL_USE_CASE

#define HAVE_SEED_COOKIE
#include "wrapper_ble.h"

#include "wrapper.h"

#include "cx.h"

#include "wrapper_nbgl.h"

#endif // WRAPPERFS_H_
