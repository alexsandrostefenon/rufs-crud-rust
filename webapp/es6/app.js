import {ServerConnectionUI} from "./ServerConnectionUI.js";
import {Crud} from "./crud.js";

const app = angular.module("app", ['ngRoute', 'ui.bootstrap']);

app.config(($controllerProvider, $routeProvider, $compileProvider, $provide) => {
	Crud.initialize($controllerProvider, $routeProvider, $compileProvider, $provide);
	$routeProvider.when('/app/login',{templateUrl:'templates/login.html', controller:'LoginController', controllerAs: "vm"});
	$routeProvider.otherwise({redirectTo: '/app/login'});
	ServerConnectionUI.changeLocationHash('/app/login');
});

import init, { add } from '../rufs_crud_rust.js';

async function run() {
	await init();
	// And afterwards we can use all the functionality defined in wasm.
	const result = add(1, 2);
	console.log(`1 + 2 = ${result}`);
	if (result !== 3)
	  throw new Error("wasm addition doesn't work!");
  }

  run();
