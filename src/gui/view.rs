use crate::gui::message::Message;
use crate::gui::state::{AppState, GuiApp};
use crate::model::Task as TodoTask;

use iced::widget::{
    Rule, button, checkbox, column, container, horizontal_space, row, scrollable, text, text_input,
};
use iced::{Background, Color, Element, Length, Theme};

pub fn root_view(app: &GuiApp) -> Element<'_, Message> {
    match app.state {
        AppState::Loading => container(text("Loading...").size(30))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        AppState::Onboarding | AppState::Settings => view_settings(app),
        AppState::Active => {
            let layout = row![
                view_sidebar(app),
                Rule::vertical(1),
                container(view_main_content(app))
                    .width(Length::Fill)
                    .center_x(Length::Fill)
            ];
            container(layout)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }
}

fn view_sidebar(app: &GuiApp) -> Element<'_, Message> {
    let cal_list = column(
        app.calendars
            .iter()
            .map(|cal| {
                let is_active = app.active_cal_href.as_ref() == Some(&cal.href);
                let btn = button(text(&cal.name).size(16))
                    .padding(10)
                    .width(Length::Fill)
                    .on_press(Message::SelectCalendar(cal.href.clone()));

                if is_active {
                    btn.style(button::primary)
                } else {
                    btn.style(button::secondary)
                }
                .into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(10)
    .width(Length::Fill);

    let settings_btn = button(row![text("Settings").size(16)].align_y(iced::Alignment::Center))
        .padding(10)
        .width(Length::Fill)
        .style(button::secondary)
        .on_press(Message::OpenSettings);

    let sidebar_inner = column![scrollable(cal_list).height(Length::Fill), settings_btn]
        .spacing(10)
        .padding(10);

    container(sidebar_inner)
        .width(200)
        .height(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(Background::Color(palette.background.weak.color)),
                ..Default::default()
            }
        })
        .into()
}

fn view_main_content(app: &GuiApp) -> Element<'_, Message> {
    let title_text = if app.loading { "Loading..." } else { "Cfait" };

    let search_input = text_input("Search...", &app.search_value)
        .on_input(Message::SearchChanged)
        .padding(5)
        .size(16);

    let header = row![
        text(title_text).size(40),
        horizontal_space(),
        search_input.width(200)
    ]
    .align_y(iced::Alignment::Center);

    let input_area = view_input_area(app);

    let is_searching = !app.search_value.is_empty();
    let filtered_tasks: Vec<(usize, &TodoTask)> = app
        .tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            if is_searching {
                t.summary
                    .to_lowercase()
                    .contains(&app.search_value.to_lowercase())
            } else {
                true
            }
        })
        .collect();

    let tasks_view = column(
        filtered_tasks
            .into_iter()
            .map(|(real_index, task)| view_task_row(app, real_index, task, is_searching))
            .collect::<Vec<_>>(),
    )
    .spacing(2);

    column![
        header,
        input_area,
        scrollable(tasks_view).height(Length::Fill)
    ]
    .spacing(20)
    .padding(20)
    .max_width(800)
    .into()
}

fn view_input_area(app: &GuiApp) -> Element<'_, Message> {
    let input_placeholder = if app.editing_uid.is_some() {
        "Edit Title..."
    } else {
        "Add task (Buy cat food !1 @weekly #chore)..."
    };

    let input_title = text_input(input_placeholder, &app.input_value)
        .on_input(Message::InputChanged)
        .on_submit(Message::SubmitTask)
        .padding(10)
        .size(20);

    if app.editing_uid.is_some() {
        let input_desc = text_input("Notes...", &app.description_value)
            .on_input(Message::DescriptionChanged)
            .on_submit(Message::SubmitTask)
            .padding(10)
            .size(16);

        let cancel_btn = button(text("Cancel").size(16))
            .style(button::secondary)
            .on_press(Message::CancelEdit);

        let save_btn = button(text("Save").size(16))
            .style(button::primary)
            .on_press(Message::SubmitTask);

        column![
            row![
                text("Editing")
                    .size(14)
                    .color(Color::from_rgb(0.7, 0.7, 1.0)),
                horizontal_space(),
                cancel_btn,
                save_btn
            ]
            .spacing(10),
            input_title,
            input_desc
        ]
        .spacing(5)
        .into()
    } else {
        column![input_title].into()
    }
}

fn view_task_row<'a>(
    app: &'a GuiApp,
    index: usize,
    task: &'a TodoTask,
    is_searching: bool,
) -> Element<'a, Message> {
    let color = match task.priority {
        1..=4 => Color::from_rgb(0.8, 0.2, 0.2),
        5 => Color::from_rgb(0.8, 0.8, 0.2),
        _ => Color::WHITE,
    };

    let indent_size = if is_searching { 0 } else { task.depth * 20 };
    let indent = horizontal_space().width(Length::Fixed(indent_size as f32));

    let summary = text(&task.summary)
        .size(20)
        .color(color)
        .width(Length::Fill);

    let mut badges = row![].spacing(5);

    for cat in &task.categories {
        badges = badges.push(
            container(text(format!("#{}", cat)).size(12).color(Color::BLACK))
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.6, 0.8, 1.0).into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(3),
        );
    }

    if task.rrule.is_some() {
        badges = badges.push(text("(R)").size(14).color(Color::from_rgb(0.6, 0.6, 1.0)));
    }

    let date_text = match task.due {
        Some(d) => text(d.format("%Y-%m-%d").to_string())
            .size(14)
            .color(Color::from_rgb(0.5, 0.5, 0.5)),
        None => text(""),
    };

    let btn_style = button::secondary;
    let has_desc = !task.description.is_empty();
    let is_expanded = app.expanded_tasks.contains(&task.uid);

    let info_btn = if has_desc {
        button(text("i").size(12))
            .style(if is_expanded {
                button::primary
            } else {
                button::secondary
            })
            .padding(5)
            .width(Length::Fixed(25.0))
            .on_press(Message::ToggleDetails(task.uid.clone()))
    } else {
        button(text("").size(12))
            .style(button::text)
            .padding(5)
            .width(Length::Fixed(25.0))
    };

    let actions = row![
        info_btn,
        button(text("+").size(14))
            .style(btn_style)
            .padding(5)
            .on_press(Message::ChangePriority(index, 1)),
        button(text("-").size(14))
            .style(btn_style)
            .padding(5)
            .on_press(Message::ChangePriority(index, -1)),
        button(text(">").size(14))
            .style(btn_style)
            .padding(5)
            .on_press(Message::IndentTask(index)),
        button(text("<").size(14))
            .style(btn_style)
            .padding(5)
            .on_press(Message::OutdentTask(index)),
        button(text("Edit").size(14))
            .style(btn_style)
            .padding(5)
            .on_press(Message::EditTaskStart(index)),
        button(text("Del").size(14))
            .style(button::danger)
            .padding(5)
            .on_press(Message::DeleteTask(index)),
    ]
    .spacing(5);

    let row_main = row![
        indent,
        checkbox("", task.completed).on_toggle(move |b| Message::ToggleTask(index, b)),
        column![
            summary,
            if !task.categories.is_empty() {
                badges
            } else {
                row![].into()
            }
        ]
        .spacing(2),
        date_text,
        actions
    ]
    .spacing(15)
    .align_y(iced::Alignment::Center);

    let padded_row = container(row_main).padding(iced::Padding {
        top: 5.0,
        right: 15.0, // Fix for scrollbar overlap
        bottom: 5.0,
        left: 5.0,
    });

    if is_expanded {
        let desc_text = text(&task.description)
            .size(14)
            .color(Color::from_rgb(0.7, 0.7, 0.7));
        let desc_row = row![
            horizontal_space().width(Length::Fixed(indent_size as f32 + 30.0)),
            desc_text
        ];
        container(column![padded_row, desc_row].spacing(5))
            .padding(5)
            .into()
    } else {
        padded_row.into()
    }
}

fn view_settings(app: &GuiApp) -> Element<'_, Message> {
    let is_settings = matches!(app.state, AppState::Settings);
    let title = text(if is_settings {
        "Settings"
    } else {
        "Welcome to Cfait"
    })
    .size(40);
    let error = if let Some(e) = &app.error_msg {
        text(e).color(Color::from_rgb(1.0, 0.0, 0.0))
    } else {
        text("")
    };

    let cal_names: Vec<String> = app.calendars.iter().map(|c| c.name.clone()).collect();
    let picker: Element<_> = if !cal_names.is_empty() && is_settings {
        column![
            text("Default Calendar:"),
            iced::widget::pick_list(
                cal_names,
                app.ob_default_cal.clone(),
                Message::ObDefaultCalChanged
            )
            .width(Length::Fill)
            .padding(10)
        ]
        .spacing(5)
        .into()
    } else {
        horizontal_space().width(0).into()
    };

    let mut buttons = row![].spacing(10);
    if is_settings {
        buttons = buttons.push(
            button("Cancel")
                .padding(10)
                .style(button::secondary)
                .on_press(Message::CancelSettings),
        );
    }
    buttons = buttons.push(
        button(if is_settings {
            "Save & Connect"
        } else {
            "Connect"
        })
        .padding(10)
        .on_press(Message::ObSubmit),
    );

    let form = column![
        text("CalDAV Server URL:"),
        text_input("https://...", &app.ob_url)
            .on_input(Message::ObUrlChanged)
            .padding(10),
        text("Username:"),
        text_input("User", &app.ob_user)
            .on_input(Message::ObUserChanged)
            .padding(10),
        text("Password:"),
        text_input("Password", &app.ob_pass)
            .on_input(Message::ObPassChanged)
            .secure(true)
            .padding(10),
        picker,
        buttons
    ]
    .spacing(15)
    .max_width(400);

    container(
        column![title, error, form]
            .spacing(20)
            .align_x(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}
