use chrono::{Duration, Local};
use plotters::prelude::*;

use crate::data::EnergyData;

pub(crate) async fn render_image(energy_data: EnergyData) -> String {
    let (from_date, to_date) = (Local::now() - Duration::days(1), Local::now());

    let data = energy_data.get_interval(from_date, to_date).await;

    let mut out_string = String::new();
    {
        let root = SVGBackend::with_string(&mut out_string, (1024, 768)).into_drawing_area(); // (OUT_FILE_NA).into_drawing_area();
        root.fill(&WHITE).unwrap();

        let mut chart = ChartBuilder::on(&root)
            .margin(5)
            .x_label_area_size(50)
            .x_label_area_size(35)
            .y_label_area_size(40)
            .right_y_label_area_size(40)
            .build_cartesian_2d(from_date..to_date, -300..5000)
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
