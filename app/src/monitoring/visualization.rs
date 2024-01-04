use hackdose_server_shared::{PlotData, Range};
use plotters::prelude::*;

pub(crate) fn render_image(data: &PlotData, range: Range, width: u32, height: u32) -> String {
    let data = data.into_iter().map(|x| x.to_tuple()).collect::<Vec<_>>();

    let mut out_string = String::new();
    {
        let root = SVGBackend::with_string(&mut out_string, (width, height)).into_drawing_area(); // (OUT_FILE_NA).into_drawing_area();
        root.fill(&WHITE).unwrap();

        let mut chart = ChartBuilder::on(&root)
            .margin(5)
            .x_label_area_size(50)
            .x_label_area_size(35)
            .y_label_area_size(40)
            .right_y_label_area_size(40)
            .build_cartesian_2d(range.from..range.to, -3000..5000)
            .unwrap();

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperMiddle);
        chart
            .configure_mesh()
            .x_labels(6)
            .y_labels(20)
            .light_line_style(&WHITE)
            .x_label_formatter(&|v| format!("{:?}", v))
            .y_label_formatter(&|v| format!("{:?}", v))
            .draw()
            .unwrap();

        chart
            .draw_series(LineSeries::new(data, GREEN.filled()))
            .unwrap();

        root.present().unwrap();
    }
    out_string
}
