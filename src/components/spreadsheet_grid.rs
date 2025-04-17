use leptos::*;

#[component]
pub fn SpreadsheetGrid() -> impl IntoView {
    let columns = ('A'..='J').collect::<Vec<_>>();
    let rows = 1..=10;

    view! {
        <table class="spreadsheet-table">
            <thead>
                <tr>
                    <th></th>
                    {columns.iter().map(|col| view! {
                        <th>
                            {col.to_string()}
                            <button class="filter-sort-button">'V'</button>
                        </th>
                    }).collect_view()}
                </tr>
            </thead>
            <tbody>
                {rows.map(|row| view! {
                    <tr>
                        <th>{row}</th>
                        {columns.iter().map(|_| view! {
                            <td><input class="cell-input" type="text" /></td>
                        }).collect_view()}
                    </tr>
                }).collect_view()}
            </tbody>
        </table>
    }
}

