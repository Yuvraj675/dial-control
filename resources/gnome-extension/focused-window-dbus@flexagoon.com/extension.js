import { Extension, gettext as _ } from 'resource:///org/gnome/shell/extensions/extension.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import St from 'gi://St';
import Clutter from 'gi://Clutter';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';

const SCROLL_PILL_ROLE = 'dial-control-scroll';

const DBUS_SCHEMA = `
<node>
    <interface name="org.gnome.shell.extensions.FocusedWindow">
        <method name="Get">
            <arg type="s" direction="out" name="window" />
        </method>
        <method name="MoveWindow">
            <arg type="s" direction="in" name="title" />
            <arg type="i" direction="in" name="x" />
            <arg type="i" direction="in" name="y" />
        </method>
        <method name="SetScrollMode">
            <arg type="b" direction="in" name="active" />
        </method>
        <signal name="FocusChanged">
            <arg type="s" name="window" />
        </signal>
    </interface>
</node>`;

function hasValidMonitor(metaWindow) {
  const windowMonitor = metaWindow.get_monitor();
  if (windowMonitor < 0) {
    return false;
  }
  const numMonitors = global.display.get_n_monitors();
  if (windowMonitor >= numMonitors) {
    return false;
  }
  return true;
}

export default class FocusedWindowDbus extends Extension {
  Get() {
    let window_list = global.get_window_actors();
    let focusedWindow = window_list.find((window) =>
      window.meta_window.has_focus()
    );

    let workspaceManager = global.workspace_manager;
    let currentmonitor = global.display.get_current_monitor();
    if (focusedWindow) {
      const validMonitor = hasValidMonitor(focusedWindow.meta_window);

      return JSON.stringify({
        title: focusedWindow.meta_window.get_title(),
        wm_class: focusedWindow.meta_window.get_wm_class(),
        wm_class_instance: focusedWindow.meta_window.get_wm_class_instance(),
        pid: focusedWindow.meta_window.get_pid(),
        id: focusedWindow.meta_window.get_id(),
        width: focusedWindow.get_width(),
        height: focusedWindow.get_height(),
        x: focusedWindow.get_x(),
        y: focusedWindow.get_y(),
        focus: focusedWindow.meta_window.has_focus(),
        in_current_workspace: focusedWindow.meta_window.located_on_workspace(
          workspaceManager.get_active_workspace()
        ),
        moveable: focusedWindow.meta_window.allows_move(),
        resizeable: focusedWindow.meta_window.allows_resize(),
        canclose: focusedWindow.meta_window.can_close(),
        canmaximize: focusedWindow.meta_window.can_maximize(),
        maximized: focusedWindow.meta_window.is_maximized(),
        canminimize: focusedWindow.meta_window.can_minimize(),
        display: focusedWindow.meta_window.get_display(),
        frame_type: focusedWindow.meta_window.get_frame_type(),
        window_type: focusedWindow.meta_window.get_window_type(),
        layer: focusedWindow.meta_window.get_layer(),
        monitor: focusedWindow.meta_window.get_monitor(),
        role: focusedWindow.meta_window.get_role(),
        area: validMonitor ? focusedWindow.meta_window.get_work_area_current_monitor() : null,
        area_all: validMonitor ? focusedWindow.meta_window.get_work_area_all_monitors() : null,
        area_cust: validMonitor ? focusedWindow.meta_window.get_work_area_for_monitor(currentmonitor) : null,
      });
    } else {
      return "{}";
    }
  }

  MoveWindow(title, x, y) {
    let window_list = global.get_window_actors();
    let winActor = window_list.find((actor) => {
      let t = actor.meta_window ? actor.meta_window.get_title() : '';
      let c = actor.meta_window ? actor.meta_window.get_wm_class() : '';
      return (t && t.includes(title)) || (c && c.includes('dial-control') && t && t.includes('Scroll'));
    });
    if (winActor && winActor.meta_window) {
      winActor.meta_window.move_frame(false, x, y);
    }
  }

  // Dial Control scroll mode: a red "SCROLL" pill in the top bar. Drawn by the Shell itself,
  // so it never takes focus from the window being scrolled.
  SetScrollMode(active) {
    if (active && !this.#scrollPill) {
      this.#scrollPill = new PanelMenu.Button(0.0, 'Dial Control Scroll Mode', true);
      const label = new St.Label({
        text: '\u2195 SCROLL',
        y_align: Clutter.ActorAlign.CENTER,
        style: 'background-color: #d71921; color: #ffffff; font-weight: bold; ' +
          'border-radius: 9px; padding: 1px 9px; font-size: 0.85em;',
      });
      this.#scrollPill.add_child(label);
      Main.panel.addToStatusArea(SCROLL_PILL_ROLE, this.#scrollPill, 0, 'right');
    } else if (!active && this.#scrollPill) {
      this.#scrollPill.destroy();
      this.#scrollPill = null;
    }
  }

  #windowTracker = null;
  #focusConnection = null;
  #scrollPill = null;

  enable() {
    this.#windowTracker = Shell.WindowTracker.get_default();
    this.#focusConnection = this.#windowTracker.connect("notify::focus-app", () => {
      this._dbus.emit_signal("FocusChanged", new GLib.Variant("(s)", [this.Get()]));
    });

    this._dbus = Gio.DBusExportedObject.wrapJSObject(DBUS_SCHEMA, this);
    this._dbus.export(
      Gio.DBus.session,
      "/org/gnome/shell/extensions/FocusedWindow"
    );
  }

  disable() {
    this.SetScrollMode(false);

    if (this.#windowTracker && this.#focusConnection) {
      this.#windowTracker.disconnect(this.#focusConnection);
      this.#focusConnection = null;
      this.#windowTracker = null;
    }

    if (this._dbus) {
      this._dbus.flush();
      this._dbus.unexport();
      delete this._dbus;
    }
  }
}
