import {CrudUiSkeleton} from "./CrudUiSkeleton.js";
import {ServerConnectionUI} from "./ServerConnectionUI.js";
// differ from CrudUiSkeleton by rufsService dependency
class CrudCommom extends CrudUiSkeleton {

	constructor(serverConnection, path, method, schema_place, property_name) {
		super(serverConnection, path, method, schema_place, property_name);
		this.rufsService = this.serverConnection.services[this.name];
		this.list = this.rufsService.list;
		this.serverConnection.addRemoteListener(this);
		this.activeTab = 0;
		this.activeSchemaType = "response";
		//this.schemaResponse = this.serverConnection.openapi.get_schema(path, method, schema_place, property_name);
	}

	process(action, params) {
		this.action = action;
		return Promise.resolve().
		then(() => {
		}).
		then(() => {
			if (action == "new") {
				if (this.activeSchemaType != "request") {
					this.activeSchemaType = "request";
					this.setSchema("post", this.activeSchemaType, this.property_name);
					this.updateFields();
				}
			} else if (this.activeSchemaType != "response") {
				//if (this.schemaResponse != undefined) {
					this.activeSchemaType = "response";
					this.setSchema(this.method, this.activeSchemaType, this.property_name);
					this.updateFields();
				//}
			}
		}).
		then(() => {
			return super.process(action, params);
		}).
		then(() => {
			let promise = undefined;

			if (action == "search") {
				this.templateModel = "./templates/crud-model_search.html";

				if (params.filter != undefined || params.filterRange != undefined || params.filterRangeMin != undefined || params.filterRangeMax != undefined) {
					promise = this.rufsService.process(action, params).
					then(() => this.list = this.rufsService.list).
					then(() => {
						if (params.filterRange != undefined) {
							for (let [fieldName, value] of Object.entries(params.filterRange)) this.setFilterRange(fieldName, value);
						}

						this.applyFilter(params.filter, params.filterRangeMin, params.filterRangeMax);
						return this.setPage(1);
					});
				} else {
					promise = Promise.resolve();
				}

				if (params.aggregate != undefined) {
					promise = promise.then(() => this.applyAggregate(params.aggregate));
				}

				if (params.sort != undefined) {
					promise = promise.then(() => this.applySort(params.sort));
				}

				if (params.pagination != undefined) {
					promise = promise.then(() => this.paginate(params.pagination));
				}
			} else if (action == "new") {
				this.templateModel = "./templates/crud-model_new.html";
				promise = this.setValues(params.overwrite, true, true);
			} else if (action == "view") {
				this.templateModel = "./templates/crud-model_view.html";
				this.primaryKey = this.rufsService.getPrimaryKey(params.primaryKey);
				promise = this.get(this.primaryKey);
			} else if (action == "edit") {
				this.templateModel = "./templates/crud-model_edit.html";
				this.primaryKey = this.rufsService.getPrimaryKey(params.primaryKey);
				promise = this.get(this.primaryKey);
			}

			return promise;
		}).then(res => {
//			this.title = this.rufsService.label;
			this.serverConnection.$scope.$apply();
			return res;
		})
	}

	// action: ["new", "update", "delete"]
	onNotify(schemaName, primaryKey, action) {
		this.applyFilter();
		this.applyAggregate();
		this.applySort();
	}

	buildLocationHash(hashPath, hashSearchObj) {
		return ServerConnectionUI.buildLocationHash(this.rufsService.path + "/" + hashPath, hashSearchObj)
	}

	goToSearch() {
		if (window.location.hash.endsWith("/search") == false) {
			window.history.back();
		}
	}
	// fieldName, 'view', item, false
    goToField(fieldName, action, obj, isGoNow) {
//    	console.log(`[${this.constructor.name}.goToField(${fieldName}, ${action})]`);
		let url = super.goToField(fieldName, action, obj, isGoNow);

		if (url == "") {
			const service = this.rufsService;
			const primaryKey = this.rufsService.getPrimaryKey(obj);
			url = ServerConnectionUI.buildLocationHash(service.path + "/" + action, {primaryKey});

			if (isGoNow == true) {
				ServerConnectionUI.changeLocationHash(url);
			}
		}

    	return url;
    }

	get(primaryKey) {
		return this.rufsService.get(primaryKey).then(response => {
			this.original = JSON.parse(JSON.stringify(response.data))
			// atualiza as strings de referência
			return this.setValues(response.data, false, false).then(() => response);
		});
	}

	remove(primaryKey) {
		if (primaryKey == undefined) {
			primaryKey = this.primaryKey;
		}

		return this.rufsService.remove(primaryKey);
	}

	update() {
		return this.rufsService.update(this.primaryKey, this.instance).
		then(response => {
			this.original = JSON.parse(JSON.stringify(response.data))
			return response;
		});
	}

	save() {
		this.primaryKey = {};
		return this.rufsService.save(this.instance).
		then(response => {
			this.original = JSON.parse(JSON.stringify(response.data))
			return response;
		});
	}

	applyAggregate(aggregate) {
		super.applyAggregate(aggregate);
		if (this.aggregateResults.length == 0) return;
		const chart = document.getElementById("aggregate-chart");
		if (chart == undefined) return;
		const ctx = chart.getContext('2d');
		const xData = Array.from(this.aggregateResults.keys());
		const yData = Array.from(this.aggregateResults.values());
		
		if (this.aggregateChartOptions == undefined) {
			this.aggregateChartOptions = {type: 'bar', data: {labels: [], datasets: [{label: "", data: []}]}};
			this.aggregateChart = new Chart(ctx, this.aggregateChartOptions);
		}
		
		let label = "";
		
		for (let fieldName in this.instanceAggregateRange) {
			let range = this.instanceAggregateRange[fieldName];
			if (range != false && range != "" && range != 0) label = label + fieldName + ",";
		}
		
		this.aggregateChartOptions.data.labels = xData;
		this.aggregateChartOptions.data.datasets[0].label = label;
		this.aggregateChartOptions.data.datasets[0].data = yData;
		this.aggregateChart.update();
	}
}

export {CrudCommom}
