/* JNI bridge into native_c/core - mirrors native_c/win32_app/main.c's
   run_pack_sample, exposed to the Kotlin Activity as
   Java_com_omnipack_app_MainActivity_packSample. */
#include <jni.h>
#include <stdio.h>
#include <string.h>

#include "omnipack/models.h"
#include "omnipack/multi_container.h"

static void append_line(char* buf, size_t buf_size, const char* line) {
    size_t used = strlen(buf);
    if (used < buf_size - 1) {
        snprintf(buf + used, buf_size - used, "%s", line);
    }
}

JNIEXPORT jstring JNICALL
Java_com_omnipack_app_MainActivity_packSample(JNIEnv* env, jobject thiz) {
    (void)thiz;

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
        snprintf(line, sizeof(line), "Container %s: %zu items, %.1f%% utilization\n",
                   c->id, c->items.len, container_volume_utilization(c));
        append_line(text, sizeof(text), line);

        for (size_t j = 0; j < c->items.len; ++j) {
            const Item* it = &c->items.data[j];
            snprintf(line, sizeof(line), "  %s @ (%.1f, %.1f, %.1f)\n",
                       it->id, it->position.x, it->position.y, it->position.z);
            append_line(text, sizeof(text), line);
        }
    }

    jstring result = (*env)->NewStringUTF(env, text);

    container_array_free(&out);
    container_free(&base);

    return result;
}
