/* Raw Win32 smoke-test UI for omnipack_core_c - mirrors native/app's Qt
   smoke test (same hardcoded sample), proving the plain-C engine + a raw
   platform-native UI work end to end. No 3D rendering yet (that's a
   follow-up task - see conductor/tasks.json). */
#include <windows.h>
#include <stdio.h>
#include <string.h>

#include "omnipack/models.h"
#include "omnipack/multi_container.h"

#define ID_PACK_BUTTON 1001
#define ID_RESULT_EDIT 1002

static HWND g_button;
static HWND g_edit;

static void append_line(char* buf, size_t buf_size, const char* line) {
    size_t used = strlen(buf);
    if (used < buf_size - 1) {
        snprintf(buf + used, buf_size - used, "%s", line);
    }
}

static void run_pack_sample(void) {
    Container base;
    container_init(&base, "C1", 100, 100, 100);

    Item items[3];
    items[0] = item_make("Box_1", 40, 40, 40); items[0].weight = 10.0;
    items[1] = item_make("Box_2", 30, 30, 30); items[1].weight = 5.0;
    items[2] = item_make("Box_3", 20, 20, 20); items[2].weight = 2.0;

    MultiContainerEngine engine;
    multi_container_engine_init(&engine, base.id, base.width, base.height, base.depth,
                                 base.max_weight, base.shape_type, VERSUS_LONGITUDINAL);

    ContainerArray out;
    container_array_init(&out);
    multi_container_engine_pack_all(&engine, items, 3, "level2", 1.0, 1, 0.0, &out);

    static char text[4096];
    text[0] = '\0';
    char line[256];

    for (size_t i = 0; i < out.len; ++i) {
        const Container* c = &out.data[i];
        snprintf(line, sizeof(line), "Container %s: %zu items, %.1f%% utilization\r\n",
                   c->id, c->items.len, container_volume_utilization(c));
        append_line(text, sizeof(text), line);

        for (size_t j = 0; j < c->items.len; ++j) {
            const Item* it = &c->items.data[j];
            snprintf(line, sizeof(line), "  %s @ (%.1f, %.1f, %.1f)\r\n",
                       it->id, it->position.x, it->position.y, it->position.z);
            append_line(text, sizeof(text), line);
        }
    }

    SetWindowTextA(g_edit, text);

    container_array_free(&out);
    container_free(&base);
}

static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    switch (msg) {
        case WM_CREATE: {
            g_button = CreateWindowExA(0, "BUTTON", "Pack Sample",
                                        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
                                        16, 16, 140, 30, hwnd, (HMENU)(INT_PTR)ID_PACK_BUTTON,
                                        NULL, NULL);
            g_edit = CreateWindowExA(WS_EX_CLIENTEDGE, "EDIT", "Click \"Pack Sample\" to run the ported C engine.",
                                      WS_CHILD | WS_VISIBLE | WS_VSCROLL | ES_MULTILINE | ES_READONLY,
                                      16, 56, 440, 280, hwnd, (HMENU)(INT_PTR)ID_RESULT_EDIT,
                                      NULL, NULL);
            HFONT font = (HFONT)GetStockObject(DEFAULT_GUI_FONT);
            SendMessage(g_button, WM_SETFONT, (WPARAM)font, TRUE);
            SendMessage(g_edit, WM_SETFONT, (WPARAM)font, TRUE);
            return 0;
        }
        case WM_SIZE: {
            RECT rc;
            GetClientRect(hwnd, &rc);
            if (g_edit) MoveWindow(g_edit, 16, 56, rc.right - 32, rc.bottom - 72, TRUE);
            return 0;
        }
        case WM_COMMAND:
            if (LOWORD(wParam) == ID_PACK_BUTTON && HIWORD(wParam) == BN_CLICKED) {
                run_pack_sample();
            }
            return 0;
        case WM_DESTROY:
            PostQuitMessage(0);
            return 0;
        default:
            return DefWindowProcA(hwnd, msg, wParam, lParam);
    }
}

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hPrevInstance; (void)lpCmdLine;

    WNDCLASSEXA wc;
    memset(&wc, 0, sizeof(wc));
    wc.cbSize = sizeof(wc);
    wc.style = CS_HREDRAW | CS_VREDRAW;
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInstance;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    wc.lpszClassName = "OmniPackWin32App";
    RegisterClassExA(&wc);

    HWND hwnd = CreateWindowExA(0, "OmniPackWin32App", "OmniPack (native C smoke test)",
                                 WS_OVERLAPPEDWINDOW,
                                 CW_USEDEFAULT, CW_USEDEFAULT, 500, 400,
                                 NULL, NULL, hInstance, NULL);
    ShowWindow(hwnd, nCmdShow);
    UpdateWindow(hwnd);

    MSG msg;
    while (GetMessage(&msg, NULL, 0, 0)) {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }
    return 0;
}
