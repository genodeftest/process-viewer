use glib::object::Cast;
use gtk::{
    self, BoxExt, ContainerExt, GridExt, Inhibit, LabelExt, ProgressBarExt,
    ScrolledWindowExt, ToggleButtonExt, Widget, WidgetExt,
};
use sysinfo::{self, ComponentExt, NetworkExt, ProcessorExt, SystemExt};

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

use graph::Graph;
use notebook::NoteBook;
use utils::RotateVec;

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn add_header(label_text: &str, parent_grid: &gtk::Grid, row_count: &mut i32) -> gtk::CheckButton {
    let check_box = gtk::CheckButton::new_with_label("Graph view");
    let label = gtk::Label::new(Some(label_text));

    parent_grid.attach(&label, 0, *row_count, 3, 1);
    parent_grid.attach(&check_box, 3, *row_count, 1, 1);
    *row_count += 1;
    check_box
}

fn create_progress_bar(non_graph_layout: &gtk::Grid, line: i32, label: &str,
                       text: &str) -> gtk::ProgressBar {
    let p = gtk::ProgressBar::new();
    let l = gtk::Label::new(Some(label));

    p.set_text(Some(text));
    p.set_show_text(true);
    non_graph_layout.attach(&l, 0, line, 1, 1);
    non_graph_layout.attach(&p, 1, line, 11, 1);
    p
}

fn format_number(mut nb: u64) -> String {
    if nb < 1000 {
        return format!("{} B", nb);
    }
    nb /= 1024;
    if nb < 100_000 {
        format!("{} kB", nb)
    } else if nb < 10_000_000 {
        format!("{} MB", nb / 1024)
    } else if nb < 10_000_000_000 {
        format!("{} GB", nb / 1_048_576)
    } else {
        format!("{} TB", nb / 1_073_741_824)
    }
}

#[allow(dead_code)]
pub struct DisplaySysInfo {
    procs: Rc<RefCell<Vec<gtk::ProgressBar>>>,
    ram: gtk::ProgressBar,
    swap: gtk::ProgressBar,
    master_grid: gtk::Grid,
    grid_row_count: i32,
    // network in usage
    in_usage: gtk::Label,
    // network out usage
    out_usage: gtk::Label,
    components: Vec<gtk::Label>,
    cpu_usage_history: Rc<RefCell<Graph>>,
    // 0 = RAM
    // 1 = SWAP
    ram_usage_history: Rc<RefCell<Graph>>,
    temperature_usage_history: Rc<RefCell<Graph>>,
    network_history: Rc<RefCell<Graph>>,
    pub ram_check_box: gtk::CheckButton,
    pub swap_check_box: gtk::CheckButton,
    pub network_check_box: gtk::CheckButton,
    pub temperature_check_box: Option<gtk::CheckButton>,
}

impl DisplaySysInfo {
    pub fn new(sys1: &Rc<RefCell<sysinfo::System>>, note: &mut NoteBook,
               win: &gtk::ApplicationWindow) -> DisplaySysInfo {

        let master_grid = gtk::Grid::new();
        let mut grid_row_count : i32 = 0;

        let mut procs = Vec::new();
        let scroll = gtk::ScrolledWindow::new(None, None);
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        let mut components = vec!();
        let mut cpu_usage_history = Graph::new(None);
        let mut ram_usage_history = Graph::new(None);
        let mut temperature_usage_history = Graph::new(Some(1.));
        let mut network_history = Graph::new(Some(1.));
        let mut check_box3 = None;

        let non_graph_layout = gtk::Grid::new();
        non_graph_layout.set_column_homogeneous(true);
        non_graph_layout.set_margin_right(5);
        let non_graph_layout2 = gtk::Grid::new();
        non_graph_layout2.set_column_homogeneous(true);
        non_graph_layout2.set_margin_right(5);
        let non_graph_layout3 = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let non_graph_layout4 = gtk::Box::new(gtk::Orientation::Vertical, 0);


        master_grid.attach(&gtk::Label::new(Some("Total CPU usage")), 0, grid_row_count, 4, 1);
        grid_row_count += 1;
        procs.push(gtk::ProgressBar::new());
        {
            let p: &gtk::ProgressBar = &procs[0];
            let s = sys1.borrow();

            p.set_margin_right(5);
            p.set_margin_left(5);
            p.set_show_text(true);
            p.set_hexpand(true);
            let processor_list = s.get_processor_list();
            if !processor_list.is_empty() {
                let pro = &processor_list[0];
                p.set_text(format!("{:.2} %", pro.get_cpu_usage() * 100.).as_str());
                p.set_fraction(f64::from(pro.get_cpu_usage()));
            } else {
                p.set_text(Some("0.0 %"));
                p.set_fraction(0.);
            }
            master_grid.attach(p, 0, grid_row_count, 4, 1);
            grid_row_count += 1;
        }


        //
        // PROCESS PART
        //
        let check_box = add_header("Process usage", &master_grid, &mut grid_row_count);
        for (i, pro) in sys1.borrow().get_processor_list().iter().skip(1).enumerate() {
            let i = i + 1;
            procs.push(gtk::ProgressBar::new());
            let p: &gtk::ProgressBar = &procs[i];
            let l = gtk::Label::new(format!("{}", i).as_str());

            p.set_text(format!("{:.2} %", pro.get_cpu_usage() * 100.).as_str());
            p.set_show_text(true);
            p.set_fraction(f64::from(pro.get_cpu_usage()));
            non_graph_layout.attach(&l, 0, i as i32 - 1, 1, 1);
            non_graph_layout.attach(p, 1, i as i32 - 1, 11, 1);
            cpu_usage_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                                   &format!("process {}", i), None);
        }
        master_grid.attach(&non_graph_layout, 0, grid_row_count, 4, 1);
        grid_row_count += 1;
        cpu_usage_history.attach_to(&master_grid, &mut grid_row_count);


        //
        // MEMORY PART
        //
        let check_box2 = add_header("Memory usage", &master_grid, &mut grid_row_count);
        let ram = create_progress_bar(&non_graph_layout2, 0, "RAM", "");
        let swap = create_progress_bar(&non_graph_layout2, 1, "Swap", "");
        master_grid.attach(&non_graph_layout2, 0, grid_row_count, 4, 1);
        grid_row_count += 1;
        ram_usage_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                               "RAM", Some(4));
        ram_usage_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                               "Swap", Some(2));
        ram_usage_history.attach_to(&master_grid, &mut grid_row_count);


        //
        // TEMPERATURES PART
        //
        if !sys1.borrow().get_components_list().is_empty() {
            check_box3 = Some(add_header("Components' temperature", &master_grid, &mut grid_row_count));
            for component in sys1.borrow().get_components_list() {
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                // TODO: add max and critical temperatures as well
                let temp = gtk::Label::new(format!("{:.1} °C",
                                                   component.get_temperature()).as_str());
                horizontal_layout.pack_start(&gtk::Label::new(component.get_label()),
                                             true, false, 0);
                horizontal_layout.pack_start(&temp, true, false, 0);
                horizontal_layout.set_homogeneous(true);
                non_graph_layout3.add(&horizontal_layout);
                components.push(temp);
                temperature_usage_history.push(RotateVec::new(iter::repeat(0f64)
                                                                   .take(61)
                                                                   .collect()),
                                               component.get_label(), None);
            }
            master_grid.attach(&non_graph_layout3, 0, grid_row_count, 4, 1);
            grid_row_count += 1;
            temperature_usage_history.attach_to(&master_grid, &mut grid_row_count);
        }


        //
        // NETWORK PART
        //
        let check_box4 = add_header("Network usage", &master_grid, &mut grid_row_count);
        // input data
        let in_usage = gtk::Label::new(format_number(0).as_str());
        let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        horizontal_layout.pack_start(&gtk::Label::new("Input data"), true, false, 0);
        horizontal_layout.pack_start(&in_usage, true, false, 0);
        horizontal_layout.set_homogeneous(true);
        non_graph_layout4.add(&horizontal_layout);
        network_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                             "Input data", None);
        // output data
        let out_usage = gtk::Label::new(format_number(0).as_str());
        let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        horizontal_layout.pack_start(&gtk::Label::new("Output data"), true, false, 0);
        horizontal_layout.pack_start(&out_usage, true, false, 0);
        horizontal_layout.set_homogeneous(true);
        non_graph_layout4.add(&horizontal_layout);
        network_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                             "Output data", None);
        master_grid.attach(&non_graph_layout4, 0, grid_row_count, 4, 1);
        grid_row_count += 1;
        network_history.attach_to(&master_grid, &mut grid_row_count);
        network_history.area.set_margin_bottom(20);


        //
        // Putting everyting into places now.
        //
        let cpu_usage_history = connect_graph(cpu_usage_history);
        let ram_usage_history = connect_graph(ram_usage_history);
        let temperature_usage_history = connect_graph(temperature_usage_history);
        let network_history = connect_graph(network_history);

        scroll.add(&master_grid);
        let scroll : Widget = scroll.upcast();
        note.create_tab("System usage", &scroll);

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram: ram.clone(),
            swap: swap.clone(),
            out_usage: out_usage.clone(),
            in_usage: in_usage.clone(),
            master_grid: master_grid,
            grid_row_count: grid_row_count,
            components: components,
            cpu_usage_history: Rc::clone(&cpu_usage_history),
            ram_usage_history: Rc::clone(&ram_usage_history),
            ram_check_box: check_box.clone(),
            swap_check_box: check_box2.clone(),
            temperature_usage_history: Rc::clone(&temperature_usage_history),
            temperature_check_box: check_box3.clone(),
            network_history: Rc::clone(&network_history),
            network_check_box: check_box4.clone(),
        };
        tmp.update_ram_display(&sys1.borrow(), false);

        check_box.clone().upcast::<gtk::ToggleButton>()
                 .connect_toggled(clone!(non_graph_layout, cpu_usage_history => move |c| {
            show_if_necessary(c, &cpu_usage_history.borrow(), &non_graph_layout);
        }));
        check_box2.clone().upcast::<gtk::ToggleButton>()
                  .connect_toggled(clone!(non_graph_layout2, ram_usage_history => move |c| {
            show_if_necessary(c, &ram_usage_history.borrow(), &non_graph_layout2);
        }));
        if let Some(ref check_box3) = check_box3 {
            check_box3.clone().upcast::<gtk::ToggleButton>()
                 .connect_toggled(clone!(non_graph_layout3, temperature_usage_history => move |c| {
                show_if_necessary(c, &temperature_usage_history.borrow(), &non_graph_layout3);
            }));
        }
        check_box4.clone().upcast::<gtk::ToggleButton>()
                  .connect_toggled(clone!(non_graph_layout4, network_history => move |c| {
            show_if_necessary(c, &network_history.borrow(), &non_graph_layout4);
        }));

        scroll.connect_show(clone!(cpu_usage_history, ram_usage_history => move |_| {
            show_if_necessary(&check_box.clone().upcast::<gtk::ToggleButton>(),
                              &cpu_usage_history.borrow(), &non_graph_layout);
            show_if_necessary(&check_box2.clone().upcast::<gtk::ToggleButton>(),
                              &ram_usage_history.borrow(), &non_graph_layout2);
            if let Some(ref check_box3) = check_box3 {
                show_if_necessary(&check_box3.clone().upcast::<gtk::ToggleButton>(),
                                  &temperature_usage_history.borrow(), &non_graph_layout3);
            }
            show_if_necessary(&check_box4.clone().upcast::<gtk::ToggleButton>(),
                              &network_history.borrow(), &non_graph_layout4);
        }));
        tmp
    }

    pub fn update_ram_display(&mut self, sys: &sysinfo::System, display_fahrenheit: bool) {
        let disp = |total, used| {
            if total < 100_000 {
                format!("{} / {} kB", used, total)
            } else if total < 10_000_000 {
                format!("{:.2} / {} MB", used as f64 / 1_024f64, total / 1_024)
            } else if total < 10_000_000_000 {
                format!("{:.2} / {} GB", used as f64 / 1_048_576f64, total / 1_048_576)
            } else {
                format!("{:.2} / {} TB", used as f64 / 1_073_741_824f64, total / 1_073_741_824)
            }
        };

        let total_ram = sys.get_total_memory();
        let used = sys.get_used_memory();
        self.ram.set_text(disp(total_ram, used).as_str());
        if total_ram != 0 {
            self.ram.set_fraction(used as f64 / total_ram as f64);
        } else {
            self.ram.set_fraction(0.0);
        }
        {
            let mut r = self.ram_usage_history.borrow_mut();
            r.data[0].move_start();
            if let Some(p) = r.data[0].get_mut(0) {
                *p = used as f64 / total_ram as f64;
            }
        }

        let total = ::std::cmp::max(sys.get_total_swap(), total_ram);
        let used = sys.get_used_swap();
        self.swap.set_text(disp(sys.get_total_swap(), used).as_str());

        let mut fraction = if total != 0 { used as f64 / total as f64 } else { 0f64 };
        if fraction.is_nan() {
            fraction = 0f64;
        }
        self.swap.set_fraction(fraction);
        {
            let mut r = self.ram_usage_history.borrow_mut();
            r.data[1].move_start();
            if let Some(p) = r.data[1].get_mut(0) {
                *p = used as f64 / total as f64;
            }
        }

        let mut t = self.temperature_usage_history.borrow_mut();
        for (pos, (component, label)) in sys.get_components_list()
                                            .iter()
                                            .zip(self.components.iter())
                                            .enumerate() {
            t.data[pos].move_start();
            if let Some(t) = t.data[pos].get_mut(0) {
                *t = f64::from(component.get_temperature());
            }
            if let Some(t) = t.data[pos].get_mut(0) {
                *t = f64::from(component.get_temperature());
            }
            if display_fahrenheit {
                label.set_text(&format!("{:.1} °F", component.get_temperature() * 1.8 + 32.));
            } else {
                label.set_text(&format!("{:.1} °C", component.get_temperature()));
            }
        }

        // network part
        let mut t = self.network_history.borrow_mut();
        self.in_usage.set_text(format_number(sys.get_network().get_income()).as_str());
        self.out_usage.set_text(format_number(sys.get_network().get_outcome()).as_str());
        t.data[0].move_start();
        *t.data[0].get_mut(0).expect("cannot get data 0") = sys.get_network().get_income() as f64;
        t.data[1].move_start();
        *t.data[1].get_mut(0).expect("cannot get data 1") = sys.get_network().get_outcome() as f64;
    }

    pub fn update_process_display(&mut self, sys: &sysinfo::System) {
        let v = &*self.procs.borrow_mut();
        let h = &mut *self.cpu_usage_history.borrow_mut();

        for (i, pro) in sys.get_processor_list().iter().enumerate() {
            v[i].set_text(format!("{:.1} %", pro.get_cpu_usage() * 100.).as_str());
            v[i].set_show_text(true);
            v[i].set_fraction(f64::from(pro.get_cpu_usage()));
            if i > 0 {
                h.data[i - 1].move_start();
                if let Some(h) = h.data[i - 1].get_mut(0) {
                    *h = f64::from(pro.get_cpu_usage());
                }
            }
        }
        h.invalidate();
        self.ram_usage_history.borrow().invalidate();
        self.temperature_usage_history.borrow().invalidate();
        self.network_history.borrow().invalidate();
    }
}

fn connect_graph(graph: Graph) -> Rc<RefCell<Graph>> {
    let area = graph.area.clone();
    let graph = Rc::new(RefCell::new(graph));
    let c_graph = Rc::clone(&graph);
    area.connect_draw(move |w, c| {
        let graph = c_graph.borrow();
        graph.draw(c,
                   f64::from(w.get_allocated_width()),
                   f64::from(w.get_allocated_height()));
        Inhibit(false)
    });
    graph
}

fn show_if_necessary<T: WidgetExt>(check_box: &gtk::ToggleButton, proc_horizontal_layout: &Graph,
                                   non_graph_layout: &T) {
    if check_box.get_active() {
        proc_horizontal_layout.show_all();
        non_graph_layout.hide();
    } else {
        non_graph_layout.show_all();
        proc_horizontal_layout.hide();
    }
}
