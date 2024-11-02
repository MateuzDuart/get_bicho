const { invoke } = window.__TAURI__.core;
import { save } from '@tauri-apps/plugin-dialog';
import { listen } from "@tauri-apps/api/event";

let isProgressActive = false;

// Obter casas de apostas
async function get_houses() {
  showLoading("Carregando casas...");
  try {
    const houses = await invoke("houses", {});
    showNotification("Casas carregadas com sucesso.", "success");
    return JSON.parse(houses);
  } catch (error) {
    showNotification("Erro ao obter as casas: " + error.message, "danger");
    return [];
  } finally {
    hideLoading();
  }
}

listen("progress", (event) => {
  setProgress(event.payload)
});

let houses_in_progress = [];

window.get_bicho_data = async function (option_element = undefined) {
  let house_name, lottery;
  const select_element = document.getElementById("house-selector");

  if (!option_element) {
    house_name = select_element.options[select_element.selectedIndex].innerHTML;
    lottery = select_element.value;
  } else {
    house_name = option_element.innerHTML;
    lottery = option_element.value;
  }

  try {

    if (lottery === "invalid" || houses_in_progress.includes(lottery)) {
      showNotification(`Casa de aposta inválida`, "danger");
      return
    }

    houses_in_progress.push(lottery);

    const get_all_time_data = document.getElementById("all-time-switch").checked;
    const last_update = document.getElementById("last-updated").innerHTML;
    const timestamp = dateToTimestamp(last_update);
    let days_without_updates = getDaysDifference(timestamp);
    
    if (days_without_updates > 1600 || get_all_time_data) {
      days_without_updates = 1600;
    }
    
    showLoading(`Carregando dados da casa ${house_name}...`);
    // await invoke("get_bichos_data", {
    //   houseName: house_name,
    //   lottery: lottery,
    //   totalDays: days_without_updates
    // });

    showNotification("Dados da casa carregados com sucesso.", "success");
    window.get_table_info()
  } catch (error) {
    showNotification(`Erro ao obter dados da casa: ${error.message}`, "danger");
  } finally {
    houses_in_progress = houses_in_progress.filter(item => item !== lottery);
    hideLoading();
  }
};

window.get_table_info = async function () {
  showLoading("Carregando informações da tabela...");
  try {
    const select_element = document.getElementById("house-selector");
    const house_name = select_element.options[select_element.selectedIndex].innerHTML;

    if (house_name === "invalid") return;

    const database_info = JSON.parse(await invoke("get_database_info", { houseName: house_name }));
    document.querySelector("header > div:nth-child(1) > span").innerHTML = database_info.total_rows;
    document.querySelector("header > div:nth-child(2) > span").innerHTML = timestampToDate(database_info.date) || "O banco de dados está vazio";
    await get_groups(house_name);
    await get_loss_sequence(house_name);
    showNotification("Informações da tabela carregadas com sucesso.", "success");
  } catch (error) {
    showNotification("Erro ao obter informações da tabela: " + error.message, "danger");
  } finally {
    hideLoading();
  }
};

window.addEventListener("DOMContentLoaded", () => {
  let houseSelect = document.querySelector("#house-selector");

  get_houses().then((houses) => {
    houses.forEach(house => {
      const option = document.createElement("option");
      option.value = house.value;
      option.innerHTML = house.name;
      houseSelect.appendChild(option);
    });
  });
});

window.export_csv = async function () {
  const select_element = document.getElementById("house-selector");
  const option_element = select_element.options[select_element.selectedIndex]
  const house_name = option_element.innerHTML;
  
  if (option_element.value === "invalid") {
    showNotification(`Casa de aposta inválida`, "danger");
    return
  }

  try {
    const filePath = await save({
      defaultPath: `${house_name}.csv`,
      filters: [
        { name: 'CSV Files', extensions: ['csv'] }
      ]
    });

    if (filePath) {
      await invoke("export_csv", { houseName: house_name, filePath });
      showNotification(`Arquivo salvo em: ${filePath}`, "success");
    } else {
      showNotification(`O usuário cancelou o diálogo.`, "danger");
      console.log('');
    }
  } catch (error) {
    showNotification('Erro ao salvar arquivo: ' + error.message, "danger");
  }
};

window.get_hours = async function (house_name) {
  try {
    return await invoke("get_hours", { houseName: house_name });
  } catch (error) {
    showNotification("Erro ao obter horários: " + error.message, "danger");
    return [];
  }
};

window.get_places = async function (house_name) {
  try {
    return await invoke("get_places", { houseName: house_name });
  } catch (error) {
    showNotification("Erro ao obter prêmios: " + error.message, "danger");
    return [];
  }
};

window.add_group = async function (house_name, data) {
  try {
    await invoke("add_group", { houseName: house_name, data });
    await window.get_groups(house_name);
    get_loss_sequence(house_name);
    showNotification("Grupo adicionado com sucesso.", "success");
  } catch (error) {
    showNotification("Erro ao adicionar grupo: " + error.message, "danger");
  }
};

window.get_groups = async function (house_name) {
  try {
    const groups = JSON.parse(await invoke("get_group", { houseName: house_name }));
    renderTable(groups);
    return groups;
  } catch (error) {
    showNotification("Erro ao obter grupos: " + error.message, "danger");
    return [];
  }
};

window.update_group = async function (house_name, data) {
  try {
    await invoke("edit_group", { houseName: house_name, data });
    await window.get_groups(house_name);
    get_loss_sequence(house_name);
    showNotification("Grupo atualizado com sucesso.", "success");
  } catch (error) {
    showNotification("Erro ao atualizar grupo: " + error.message, "danger");
    throw error
  }
};

window.delete_group = async function (id) {
  const select_element = document.getElementById("house-selector");
  const house_name = select_element.options[select_element.selectedIndex].innerHTML;
  
  try {
    await invoke("delete_group", { houseName: house_name, id: parseInt(id) });
    await window.get_groups(house_name);
    get_loss_sequence(house_name);
    showNotification("Grupo deletado com sucesso.", "success");
  } catch (error) {
    showNotification("Erro ao excluir grupo: " + error.message, "danger");
  }
};

async function get_loss_sequence(house_name) {
  try {
    const loss_sequences = await invoke("get_loss_sequence", { houseName: house_name });
    renderRecommendations(loss_sequences);
  } catch (error) {
    showNotification("Erro ao obter sequência de derrota: " + error.message, "danger");
  }
}

function renderRecommendations(recommendations) {
  const minLossSequence = 8;
  const tableBody = document.querySelector("#recomendationTable tbody");

  tableBody.innerHTML = "";

  const sortedRecommendations = recommendations.sort((a, b) => b.loss_sequence - a.loss_sequence);

  for (const rec of sortedRecommendations) {
    const row = document.createElement("tr");
    const cellClass = rec.loss_sequence > minLossSequence ? "green-bg" : "red-bg";

    const lossSeqCell = document.createElement("td");
    lossSeqCell.textContent = rec.loss_sequence;
    lossSeqCell.classList.add(cellClass);
    row.appendChild(lossSeqCell);

    const hourCell = document.createElement("td");
    hourCell.textContent = rec.hour;
    hourCell.classList.add(cellClass);
    row.appendChild(hourCell);

    const placeCell = document.createElement("td");
    placeCell.textContent = rec.place;
    placeCell.classList.add(cellClass);
    row.appendChild(placeCell);

    const groupCell = document.createElement("td");
    groupCell.textContent = rec.group;
    groupCell.classList.add(cellClass);
    row.appendChild(groupCell);

    tableBody.appendChild(row);
  }
}

function getDaysDifference(timestamp) {
  const dateFromTimestamp = new Date(timestamp * 1000);
  const currentDate = new Date();
  const differenceInMillis = currentDate - dateFromTimestamp;
  return Math.floor(differenceInMillis / (1000 * 60 * 60 * 24));
}

function timestampToDate(timestamp) {
  const date = new Date(timestamp * 1000);
  const day = String(date.getDate()).padStart(2, '0');
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const year = date.getFullYear();
  return `${day}-${month}-${year}`;
}

function dateToTimestamp(dateString) {
  const [day, month, year] = dateString.split('-');
  const date = new Date(year, month - 1, day);
  return Math.floor(date.getTime() / 1000);
}

function renderTable(groups) {
  const tableBody = document.querySelector("#groupTable tbody");
  tableBody.innerHTML = "";

  for (const group of groups) {
    const row = document.createElement("tr");

    const hourCell = document.createElement("td");
    hourCell.textContent = group.hour;
    row.appendChild(hourCell);

    const placeCell = document.createElement("td");
    placeCell.textContent = group.place;
    row.appendChild(placeCell);

    const groupCell = document.createElement("td");
    groupCell.textContent = group.group.join(", ");
    row.appendChild(groupCell);

    const actionsCell = document.createElement("td");
    actionsCell.innerHTML = `
      <button class="btn btn-sm btn-warning" id="${group.id}" onclick="edit_group(this)">Editar</button>
      <button class="btn btn-sm btn-danger" id="${group.id}" onclick="window.delete_group(${group.id})">Excluir</button>
    `;
    row.appendChild(actionsCell);

    tableBody.appendChild(row);
  }
}

window.showLoading = function(message = "Carregando...") {
  const loadingBox = document.createElement("div");
  loadingBox.className = `loading-box position-fixed top-0 start-0 w-100 h-100 d-flex justify-content-center align-items-center`;
  loadingBox.innerHTML = `
    <div class="spinner-border text-primary" role="status" id="spinner">
      <span class="visually-hidden">${message}</span>
    </div>
    <div id="progressContainer" class="d-none">
      <div class="progress" style="width: 100%">
        <div id="progressBar" class="progress-bar progress-bar-striped bg-info" role="progressbar" style="width: 0%">0%</div>
      </div>
    </div>
    <span class="ms-3" id="loadingMessage">${message}</span>
  `;
  loadingBox.id = "loadingBox";
  document.body.appendChild(loadingBox);
}

function hideLoading() {
  const loadingBox = document.getElementById("loadingBox");
  if (loadingBox) loadingBox.remove();
  isProgressActive = false;
}

function showNotification(message, type = "info") {
  const alertBox = document.createElement("div");
  alertBox.className = `alert alert-${type} alert-dismissible fade show position-fixed top-0 end-0 m-3`;
  alertBox.style.zIndex = "1055"
  alertBox.role = "alert";
  alertBox.innerHTML = `
    ${message}
    <button type="button" class="btn-close" data-bs-dismiss="alert" aria-label="Close"></button>
  `;

  document.body.appendChild(alertBox);

  setTimeout(() => {
    alertBox.classList.remove("show");
    alertBox.addEventListener("transitionend", () => alertBox.remove());
  }, 5000);
}

window.setProgress = function(percent) {
  const spinner = document.getElementById("spinner");
  const progressContainer = document.getElementById("progressContainer");
  const progressBar = document.getElementById("progressBar");

  if (!progressBar) return;

  if (!isProgressActive) {
    spinner.classList.add("d-none");
    progressContainer.classList.remove("d-none");
    isProgressActive = true;
  }

  // Define a largura e o texto do progresso
  percent = Math.min(100, Math.max(0, percent)); // Limita entre 0 e 100
  progressBar.style.width = `${percent}%`;
  progressBar.textContent = `${percent}%`;

  // Finaliza e esconde o carregamento automaticamente se atingir 100%
  if (percent === 100) {
    hideLoading()
  }
}