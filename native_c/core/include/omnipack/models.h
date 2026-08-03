#ifndef OMNIPACK_MODELS_H
#define OMNIPACK_MODELS_H
/* Plain-C port of native/core/include/omnipack/models.hpp
   (itself a port of src/omnipack/core/models.py) */

#include <stddef.h>

#define OMNIPACK_ID_MAX 64

typedef enum {
    ROT_W_H_D = 0,
    ROT_H_W_D = 1,
    ROT_H_D_W = 2,
    ROT_D_H_W = 3,
    ROT_D_W_H = 4,
    ROT_W_D_H = 5
} Rotation;

typedef enum {
    STRAT_NONE = 0,
    STRAT_FIFO = 1,
    STRAT_LIFO = 2
} LoadingStrategy;

typedef enum {
    VERSUS_LONGITUDINAL = 0,
    VERSUS_LATERAL = 1,
    VERSUS_FLOOR_FIRST = 2,
    VERSUS_WALL_BUILDING = 3,
    VERSUS_CORNER_FIRST = 4
} PackingVersus;

typedef enum {
    SHAPE_BOX = 0,
    SHAPE_SPHERE = 1,
    SHAPE_CYLINDER = 2,
    SHAPE_TETRAHEDRON = 3
} ShapeType;

typedef struct {
    double x, y, z;
} Vec3;

typedef struct {
    char id[OMNIPACK_ID_MAX];
    double width, height, depth;
    double weight;
    double max_stack_weight;
    LoadingStrategy strategy;
    int stop_id;
    Rotation allowed_rotations[6];
    int allowed_rotations_count;
    ShapeType shape_type;

    /* Internal state, set by the engine while packing. */
    Vec3 position;
    Rotation rotation;
} Item;

typedef struct {
    Item* data;
    size_t len;
    size_t cap;
} ItemArray;

void item_array_init(ItemArray* arr);
void item_array_push(ItemArray* arr, Item item);
void item_array_free(ItemArray* arr);
/* Copies `count` items from `src` into a freshly-initialized `dst`. */
void item_array_copy_from(ItemArray* dst, const Item* src, size_t count);

/* Item(id, w, h, d): weight=0, max_stack_weight=1e6, strategy=NONE, stop_id=0,
   all 6 rotations allowed, shape=BOX - matches models.py's dataclass defaults. */
Item item_make(const char* id, double w, double h, double d);
double item_volume(const Item* it);
/* Writes {w,h,d} for the item's current `rotation` into out[3]. */
void item_get_dimension(const Item* it, double out[3]);
int item_allows_rotation(const Item* it, Rotation r);

typedef struct {
    char id[OMNIPACK_ID_MAX];
    double width, height, depth;
    double max_weight;
    ShapeType shape_type;
    ItemArray items;
} Container;

typedef struct {
    double volume_utilization;
    double weight_utilization;
    double total_weight;
    Vec3 center_of_mass;
    double stability_score; /* Placeholder, matches models.py */
    int item_count;
} ContainerStats;

/* Container(id, w, h, d): max_weight=1e6, shape=BOX - matches models.py's defaults. */
void container_init(Container* c, const char* id, double w, double h, double d);
void container_init_full(Container* c, const char* id, double w, double h, double d,
                          double max_weight, ShapeType shape);
void container_free(Container* c);

double container_volume(const Container* c);
double container_remaining_volume(const Container* c);
double container_volume_utilization(const Container* c);
ContainerStats container_calculate_stats(const Container* c);

/* Stop-aware accessibility: an item is blocked if another item sitting
   closer to the door (higher Z) needs a LATER stop (higher stop_id) and
   overlaps it in X-Y - i.e. it must be moved before this item can come out.
   Score is the percentage of items that are NOT blocked. */
double container_calculate_accessibility(const Container* c);

#endif /* OMNIPACK_MODELS_H */
