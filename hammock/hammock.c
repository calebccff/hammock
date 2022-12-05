
#include <stdio.h>

#include <wayland-client.h>
#include <wlr-foreign-toplevel-management-unstable-v1-client-protocol.h>

#include <cas/cutils.h>

#include "hammock.h"

struct hammock_wl {
	struct wl_display *display;
	struct wl_registry *registry;
	struct wl_compositor *compositor;
	struct wl_shell *shell;
	struct zwlr_foreign_toplevel_manager_v1 *toplevel_manager;
	bool exit;
};

static void lh_registry_global(void *data, struct wl_registry *registry,
			       uint32_t id, const char *interface, uint32_t version)
{
	struct hammock_wl *h = data;

	if (id == 1)
		log_debug("%48s | %4s | %4s", "interface", "id", "version");

	log_debug("%48s | %4u | %4u", interface, id, version);

	if (strcmp(interface, "wl_compositor") == 0) {
		log_info("found compositor");
		h->compositor = wl_registry_bind(registry, id, &wl_compositor_interface, 1);
	} else if (strcmp(interface, "wl_shell") == 0) {
		log_info("found shell");
		h->shell = wl_registry_bind(registry, id, &wl_shell_interface, 1);
	} else if (strcmp(interface, zwlr_foreign_toplevel_manager_v1_interface.name) == 0) {
		log_info("found toplevel manager");
		h->toplevel_manager = wl_registry_bind(registry, id, &zwlr_foreign_toplevel_manager_v1_interface, 1);
	}
}

static void lh_registry_global_remove(void *data, struct wl_registry *registry, uint32_t id)
{
	log_debug("global remove: %d", id);
}

static const struct wl_registry_listener registry_listener = {
	.global = lh_registry_global,
	.global_remove = lh_registry_global_remove
};

static void lh_toplevel_title(void *data, struct zwlr_foreign_toplevel_handle_v1 *toplevel,
			      const char *title)
{
	log_info("toplevel %zu title: %s", (void*)toplevel, title);
}

static void lh_toplevel_app_id(void *data, struct zwlr_foreign_toplevel_handle_v1 *toplevel,
			       const char *app_id)
{
	log_info("toplevel %zu app id: %s", (void*)toplevel, app_id);
}

static void lh_toplevel_output_enter(void *data, struct zwlr_foreign_toplevel_handle_v1 *toplevel,
				     struct wl_output *output)
{
	log_info("toplevel %zu output enter", (void*)toplevel);
}

static void lh_toplevel_output_leave(void *data, struct zwlr_foreign_toplevel_handle_v1 *toplevel,
				     struct wl_output *output)
{
	log_info("toplevel %zu output leave", (void*)toplevel);
}

static void lh_toplevel_state(void *data, struct zwlr_foreign_toplevel_handle_v1 *toplevel,
			      struct wl_array *state)
{
	log_info("toplevel %zu state", (void*)toplevel);
	print_hex_dump("", state->data, state->size);
}

static void lh_toplevel_done(void *data, struct zwlr_foreign_toplevel_handle_v1 *toplevel)
{
	log_info("toplevel %zu done", (void*)toplevel);
}

static void lh_toplevel_closed(void *data, struct zwlr_foreign_toplevel_handle_v1 *toplevel)
{
	log_info("toplevel %zu closed", (void*)toplevel);
}

static const struct zwlr_foreign_toplevel_handle_v1_listener toplevel_listener = {
	.title = lh_toplevel_title,
	.app_id = lh_toplevel_app_id,
	.output_enter = lh_toplevel_output_enter,
	.output_leave = lh_toplevel_output_leave,
	.state = lh_toplevel_state,
	.done = lh_toplevel_done,
	.closed = lh_toplevel_closed
};

static void lh_toplevel(void *data, struct zwlr_foreign_toplevel_manager_v1 *toplevel_manager,
			struct zwlr_foreign_toplevel_handle_v1 *toplevel)
{
	log_info("toplevel");
	struct hammock_wl *h = data;
	zwlr_foreign_toplevel_handle_v1_add_listener(toplevel, &toplevel_listener, h);
}

static void lh_toplevel_finished(void *data, struct zwlr_foreign_toplevel_manager_v1 *toplevel_manager)
{
	log_info("toplevel finished");
}

static const struct zwlr_foreign_toplevel_manager_v1_listener toplevel_manager_listener = {
	.toplevel = lh_toplevel,
	.finished = lh_toplevel_finished
};

static void event_queue_thread(struct hammock_wl *h)
{
	while (!h->exit) {
		wl_display_dispatch(h->display);
		wl_display_roundtrip(h->display);
	}
}

int lh_init() {
	struct hammock_wl *h = zalloc(sizeof(struct hammock_wl));

	c_workqueue_init();

	h->display = wl_display_connect(NULL);
	if (!h->display) {
		log_fatal("Can't connect to Wayland server");
		return -1;
	}
	log_debug("connected to wayland server");

	struct wl_registry *registry = wl_display_get_registry(h->display);
	wl_registry_add_listener(registry, &registry_listener, h);

	if (!h->toplevel_manager) {
		log_fatal("Can't find toplevel_manager");
		free(h);
		return -1;
	}

	zwlr_foreign_toplevel_manager_v1_add_listener(h->toplevel_manager, &toplevel_manager_listener, h);

	wl_display_disconnect(h->display);
	log_info("disconnected from wayland server");
	free(h);

	return 0;
}
