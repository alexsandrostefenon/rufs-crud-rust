import {Utils} from "./Utils.js";
import {DataStoreItem, Filter} from "./DataStore.js";
import {ServerConnectionUI} from "./ServerConnectionUI.js";

// differ of DataStoreItem by UI features, serverConnection and field.$ref dependencies
class CrudUiSkeleton extends DataStoreItem {

	static calcPageSize() {
		let pageSize;
		let avaiableHeight = window.innerHeight || document.documentElement.clientHeight || document.body.clientHeight;
		const style = window.getComputedStyle(document.getElementById("header"));
		avaiableHeight -= parseFloat(style.height);
		const rowHeight = parseFloat(style.fontSize) * 2.642857143;
    	pageSize = Math.trunc(avaiableHeight / rowHeight);
    	pageSize -= 4;
    	return pageSize;
	}

	updateFields() {
		for (let fieldName in this.properties) {
			let field = this.properties[fieldName];
			field.filter = {};
			field.htmlType = "text";
			field.htmlStep = "any";

			if (field.type == "boolean") {
				field.htmlType = "checkbox";
			} else if (field.type == "integer") {
				field.htmlType = "number";
				field.htmlStep = "1";
			} else if (field.type == "number") {
				field.htmlType = "number";
				
				if (field.precision == 1) {
					field.htmlStep = "0.1";
				} else if (field.precision == 2) {
					field.htmlStep = "0.01";
				} else {
					field.htmlStep = "0.001";
				}
			} else if (field.type == "date") {
				field.htmlType = "date";
			} else if (field.type == "time") {
				field.htmlType = "time";
			} else if (field.format == "date-time") {
				field.htmlType = "datetime-local";
			}

			if (field.enum == undefined && field.enumLabels == undefined && field.type == "string" && field.maxLength == 1 && (field.default == "S" || field.default == "N")) {
				field.filterResults = field.enum = ["S", "N"];
				field.filterResultsStr = field.enumLabels = ["Sim", "Não"];
			}

			if (field.htmlType == "number" || field.htmlType.includes("date") || field.htmlType.includes("time")) {
				field.htmlTypeIsRangeable = true;
			} else {
				field.htmlTypeIsRangeable = false;
			}
			
			if (field.label == undefined) {
				if (field.description != undefined && field.description.length <= 30) {
					field.label = field.description;
				} else {
					let label = this.dataStoreManager.convertCaseAnyToLabel(fieldName);
					field.label = label;
				}
			}

			if (field.flags != null && Array.isArray(field.flags) == false) {
				field.flags = field.flags.split(",");
				field.htmlTypeIsRangeable = false;
			}

			if (field.enum != undefined) {
				if (Array.isArray(field.enum) == false) field.enum = field.enum.split(",");
				field.htmlTypeIsRangeable = false;
			}

			if (field.enumLabels != undefined) {
				if (Array.isArray(field.enumLabels) == false) field.enumLabels = field.enumLabels.split(",");
				field.htmlTypeIsRangeable = false;
			}
			
			if (field.$ref != undefined) {
				field.htmlTypeIsRangeable = false;
			}
		}
	}

	constructor(serverConnection, path, method, schema_place, property_name, selectCallback) {
		super(serverConnection, path, method, schema_place, property_name, []);
		this.serverConnection = serverConnection;
		//this.translation = serverConnection.translation;

		if (property_name == null) {
			this.formId = this.name + "Form";
		} else {
			this.formId = `${this.name}-${this.property_name}-Form`;
		}

		this.selectCallback = selectCallback;
		this.listItemCrud = [];
		this.listItemCrudJson = [];
		this.listCrudObjJson = [];
		this.listCrudJsonArray = [];
		this.listCrudObjJsonResponse = [];
		this.updateFields();
//		this.title = "loading...";
	}

	buildFieldFilterResults() {
		const updateReferenceList = fieldName => {
			const field = this.properties[fieldName];
			let promise = Promise.resolve();

			if (field.$ref != undefined) {
				const service = this.serverConnection.getSchema(field.$ref);

				if (service != undefined) {
					promise = this.serverConnection.getDocuments(service, service.list);
				}
			}

			return promise;
		}

		const next = (propertyNames, index) => {
			if (index >= propertyNames.length) return Promise.resolve();
			const fieldName = propertyNames[index];
			return updateReferenceList(fieldName).then(() => next(propertyNames, ++index));
		}

		console.log(`buildFieldFilterResults :`, this.properties);
		return next(Object.keys(this.properties), 0).
		then(() => {
			// faz uma referencia local a field.filterResultsStr, para permitir opção filtrada, sem alterar a referencia global
			for (let [fieldName, field] of Object.entries(this.properties)) {
				if (field.$ref != undefined) {
					const rufsService = this.serverConnection.getForeignService(this, fieldName);

					if (rufsService != undefined) {
						if (Object.entries(field.filter).length == 0) {
							const pos = field.$ref.indexOf("?");

							if (pos >= 0 && Qs != undefined) {
								const primaryKey = Qs.parse(field.$ref.substring(pos), {ignoreQueryPrefix: true, allowDots: true});

								for (const [fieldName, value] of Object.entries(primaryKey)) {
									if (typeof(value) == "string" && value.startsWith("*") == true) {
										field.filter[fieldName] = this.openapi.copy_value(this.path, this.method, this.schema_place, fieldName, value.substring(1));
									}
								}
							}
						}

						if (Object.entries(field.filter).length > 0) {
							field.filterResults = [];
							field.filterResultsStr = [];

							for (let i = 0; i < rufsService.list.length; i++) {
								let candidate = rufsService.list[i];

								if (Filter.matchObject(field.filter, candidate, (a,b,fieldName) => a == b, false)) {
									field.filterResults.push(candidate);
									const str = rufsService.listStr[i];

									if (field.filterResultsStr.indexOf(str) < 0)
										field.filterResultsStr.push(str);
									else
										console.error(`[${this.constructor.name}.buildFieldFilterResults(${this.name})] : already exists string in filterResultsStr :`, str);
								}
							}
						} else {
							field.filterResults = rufsService.list;
							field.filterResultsStr = rufsService.listStr;
						}
					} else {
						console.warn("don't have acess to service ", field.$ref);
						field.filterResults = [];
						field.filterResultsStr = [];
					}
				} else if (field.enum != undefined) {
					field.filterResults = field.enum;
					console.log(`[${constructor.name}.buildFieldFilterResults()] ${fieldName}.filterResults = `, field.filterResults);

					if (field.enumLabels != undefined) {
						field.filterResultsStr = field.enumLabels;
					} else {
						field.filterResultsStr = field.enum;
					}
				}

				if (field.htmlType.includes("date")) {
					field.filterRangeOptions = [
						" hora corrente ", " hora anterior ", " uma hora ",
						" dia corrente ", " dia anterior ", " um dia ",
						" semana corrente ", " semana anterior ", " uma semana ", 
						" quinzena corrente ", " quinzena anterior ", " uma quinzena ",
						" mês corrente ", " mês anterior ", " um mês ",
						" ano corrente ", " ano anterior ", " um ano "
					];

					field.aggregateRangeOptions = ["", "hora", "dia", "mês", "ano"];
				}
			}
		});
	}

	process(action, params) {
		this.action = action;

		for (let [fieldName, property] of Object.entries(this.properties)) {
			if (property.type == "object" && property.properties != undefined && property.hiden != true) {
				if (this.listItemCrudJson.find(crudUi => crudUi.name == fieldName) != undefined) continue;
				if (this.listCrudObjJson.find(crudUi => crudUi.name == fieldName) != undefined) continue;
				const list = [""];//this.openapi.get_dependencies(this.path, this.method, this.schema_place, fieldName);

				if (list.length == 1) {
					// 	constructor(parent, properties, fieldNameExternal, title, serverConnection) {
					this.listItemCrudJson.push(new CrudUiSkeleton.CrudItemJson(this.serverConnection, this.path, this.method, this.schema_place, fieldName, null, this, property.title));
				} else {
					// 	constructor(parent, properties, fieldNameExternal, title, serverConnection) {
					this.listCrudObjJson.push(new CrudUiSkeleton.CrudObjJson(this, property.properties, fieldName, property.title, this.serverConnection));
				}
			} else if (property.type == "array" && property.items != undefined && property.items.type == "object" && property.hiden != true) {
				// serverConnection, path, method, schema_place, property_name, selectCallback, parent, options
				this.listCrudJsonArray.push(new CrudUiSkeleton.CrudJsonArray(this.serverConnection, this.path, this.method, this.schema_place, fieldName, null, this, {"action": action}));
			}
		}

		for (let [fieldName, field] of Object.entries(this.properties)) field.filter = {};
		return super.process(action, params).then(res => {
			return this.buildFieldFilterResults().then(() => {
				this.serverConnection.$scope.$apply();
				return res;
			});
		});
	}

	searchSelect(fieldName, obj) {
//		console.log(`${this.constructor.name}.searchSelect(${fieldName}, ${JSON.stringify(obj)})`);
		if (this.serverConnection.selectOut == null) this.serverConnection.selectOut = {};
		this.serverConnection.selectOut[fieldName] = obj;
		window.history.back();
	}
	// fieldName, 'view', item, false
    goToField(fieldName, action, obj, isGoNow) {
    	//console.log(`[${this.constructor.name}.goToField(${fieldName}, ${action}, ${JSON.stringify(obj)})] :`, JSON.stringify(item));
    	const field = this.properties[fieldName];
		if (field.$ref == undefined) return obj != null && obj[fieldName] != null && typeof(obj[fieldName]) == "string" && obj[fieldName].startsWith("#") == true ? obj[fieldName] : "";
		// vm.goToField(fieldName, 'search', vm.instance)
		const item = this.openapi.get_primary_key_foreign(this.path, fieldName, obj);
    	console.log(`[this.openapi.get_primary_key_foreign("${this.path}", "${fieldName}", json!(${JSON.stringify(obj)}))] :`, JSON.stringify(item));
		const service = this.serverConnection.getSchema(item.schema);
		const queryObj = {};

		if (action == "search" && isGoNow == true) {
			queryObj.selectOut = fieldName;
			queryObj.filter = {};

			if (item.is_unique_key != true) {
				for (let [fieldName, value] of Object.entries(item.primary_key))
					if (value != null) queryObj.filter[fieldName] = value;
			}

			this.serverConnection.useHistoryState = true;
			window.history.replaceState(this.instance, "Edited values");
		} else {
			queryObj.primaryKey = item.primary_key;
		}

		const url = ServerConnectionUI.buildLocationHash(service.path + "/" + action, queryObj);

		if (url == "") {
			
		} else if (url != "" && isGoNow == true) {
    		window.location.assign(url);
    	}

    	return url;
    }

	setValues(obj, enableDefault, enableNull) {
		if ((this.action == "new" || this.action == "edit") && this.serverConnection.selectOut != null && this.serverConnection.useHistoryState == true && window.history.state != null) {
			if (obj == null) obj = {};

			for (let [fieldName, property] of Object.entries(this.properties)) {
				obj[fieldName] = this.serverConnection.selectOut[fieldName] || window.history.state[fieldName] || obj[fieldName];
			}
		}

		return super.setValues(obj, enableDefault, enableNull).
		then(() => {
			// fieldFirst is used in form_body html template
			this.fieldFirst = undefined;
			const list = Object.entries(this.properties);
			let filter = list.filter(([fieldName, field]) => field.hiden != true && field.readOnly != true && field.essential == true && field.type != "object" && field.type != "array" && this.instance[fieldName] == undefined);
			if (filter.length == 0) filter = list.filter(([fieldName, field]) => field.hiden != true && field.readOnly != true && field.essential == true && field.type != "object" && field.type != "array");
			if (filter.length == 0) filter = list.filter(([fieldName, field]) => field.hiden != true && field.readOnly != true && field.essential == true);
			if (filter.length == 0) filter = list.filter(([fieldName, field]) => field.hiden != true && field.readOnly != true);
			if (filter.length == 0) filter = list.filter(([fieldName, field]) => field.hiden != true);
//			if (filter.length > 0) this.fieldFirst = filter[0][0];
			// TODO : transferir para classe pai ou primeiro ancestral com referência aos this.list*Crud
			const next = list => {
				if (list.length == 0) return obj;
				const crudXXX = list.shift();
				return crudXXX.get(this.instance).then(() => next(list));
			};

			const listCrud = [];
			Array.prototype.push.apply(listCrud, this.listItemCrudJson);
			Array.prototype.push.apply(listCrud, this.listCrudObjJson);
			Array.prototype.push.apply(listCrud, this.listCrudJsonArray);
			return next(listCrud);
		}).
		then(() => {
			this.serverConnection.$scope.$apply();
		});
	}

	clearForm() {
		this.serverConnection.selectOut = {};
		this.serverConnection.useHistoryState = false;
		return this.clear();
	}

	paginate(params) {
		if (params == undefined) params = {};
		if (params.pageSize == undefined) params.pageSize = CrudUiSkeleton.calcPageSize();
		if (params.pageSize < 10) params.pageSize = 10;
		return super.paginate(params).then(() => this.setPage(1));
	}

    setPage(page) {
    	this.pagination.setPage(page);
		const service = this.serverConnection.getSchema(this.name);
		let promise;

    	if (service != undefined) {
			promise = this.serverConnection.getDocuments(service, this.pagination.listPage);
    	} else {
    		promise = Promise.resolve();
    	}

    	return promise;
	}

    validateFieldChange(fieldName, newValue, oldValue) {
		console.log(`[CrudUiSkeleton(${this.constructor.name}).validateFieldChange(fieldName=${fieldName}, newValue=${newValue}, oldValue=${oldValue})] this.instance[${fieldName}] = ${this.instance[fieldName]}`);
		let ret = true;

		if (ret == true) {
			let stateObj = window.history.state;
			if (stateObj == null) stateObj = {};
			stateObj[fieldName] = newValue == oldValue ? this.instance[fieldName] : newValue;
//			window.history.replaceState(stateObj, "Edited values");
		}

		return ret;
    }

	parseValue(fieldName, instance) {
		// faz o inverso da funcao strAsciiHexToFlags
		const flagsToStrAsciiHex = flags => {
			let value = 0;
	
			for (let i = 0; i < flags.length; i++) {
				let flag = flags[i];
				let bit = 1 << i;
	
				if (flag == true) {
					value |= bit;
				}
			}
	
			let strAsciiHex = value.toString(16);
			return strAsciiHex;
		}

		const field = this.properties[fieldName];

		if (field.flags != null) {
			instance[fieldName] = Number.parseInt(flagsToStrAsciiHex(this.instanceFlags[fieldName]), 16);
		} else {
			let pos = field.filterResultsStr.indexOf(field.externalReferencesStr);
			
			if (pos >= 0) {
				const oldValue = instance[fieldName];
				let newValue;

				if (field.$ref != undefined) {
					const foreignData = field.filterResults[pos];
					console.log(`[parseValue(fieldName, instance)] openapi.get_foreign_key("${this.name}", "${fieldName}", json!(${JSON.stringify(foreignData)}));`);
					const foreignKey = this.openapi.get_foreign_key(this.name, fieldName, foreignData);
					newValue = foreignKey.get(fieldName);
				} else if (field.enum != undefined) {
					newValue = field.filterResults[pos];
				}

				if (typeof newValue == "string")
					newValue = newValue.trimEnd();

				if (this.validateFieldChange(fieldName, newValue, oldValue) == true) {
					instance[fieldName] = newValue;
				}
			}
		}
	}

}

export {CrudUiSkeleton}
